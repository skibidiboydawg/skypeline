//! Avatar image processing (circular anti-aliased rendering + full PNG alpha
//! transparency preservation) and a two-level (memory + disk) avatar cache.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use egui::TextureHandle;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder};
use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageFormat};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;

use crate::events::UiEvent;

const CACHE_DIR: &str = "skypeline-cache/avatars";
const MAX_CACHED_FILES: usize = 100;
const AVATAR_SIZE: u32 = 128;
const MAX_AVATAR_BYTES: usize = 96 * 1024;
const QUALITIES: [u8; 5] = [82, 72, 62, 52, 42];
/// Request key used when the caller does not know the avatar hash yet.
const UNKNOWN_HASH_KEY: &str = "*";

struct CachedAvatar {
    hash: Option<String>,
    texture: TextureHandle,
}

/// Memory textures plus the set of `(user_id, hash)` pairs already asked for.
pub struct AvatarCache {
    textures: HashMap<i64, CachedAvatar>,
    requested: HashSet<(i64, String)>,
}

impl AvatarCache {
    pub fn new() -> Self {
        prune_disk_cache(MAX_CACHED_FILES);
        Self {
            textures: HashMap::new(),
            requested: HashSet::new(),
        }
    }

    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        api: &crate::api::ApiClient,
        token: Option<&str>,
        user_id: i64,
        hash: Option<&str>,
        tx: &mpsc::Sender<UiEvent>,
    ) -> Option<TextureHandle> {
        let hash = hash.filter(|h| !h.is_empty());

        if let Some(cached) = self.textures.get(&user_id) {
            match hash {
                None => return Some(cached.texture.clone()),
                Some(h) if cached.hash.as_deref() == Some(h) => {
                    return Some(cached.texture.clone())
                }
                Some(_) => {}
            }
        }

        if let Some(h) = hash {
            if let Some(texture) = load_disk_texture(ctx, user_id, h) {
                self.textures.insert(
                    user_id,
                    CachedAvatar {
                        hash: Some(h.to_string()),
                        texture: texture.clone(),
                    },
                );
                return Some(texture);
            }
        }

        if let Some(token) = token {
            let key = hash.unwrap_or(UNKNOWN_HASH_KEY).to_string();
            if self.requested.insert((user_id, key)) {
                spawn_fetch(api.clone(), token.to_string(), user_id, tx.clone());
            }
        }

        None
    }

    pub fn on_loaded(
        &mut self,
        ctx: &egui::Context,
        user_id: i64,
        hash: String,
        mime: String,
        data_base64: String,
    ) -> Option<TextureHandle> {
        if mime != "image/jpeg" && mime != "image/png" {
            tracing::debug!("avatar {user_id} has mime '{mime}', sniffing contents");
        }

        let bytes = match STANDARD.decode(data_base64.as_bytes()) {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::warn!("invalid base64 for avatar {user_id}: {err}");
                return None;
            }
        };
        if bytes.len() > MAX_AVATAR_BYTES {
            tracing::warn!(
                "oversized avatar {user_id}: {} bytes (limit {MAX_AVATAR_BYTES})",
                bytes.len()
            );
            return None;
        }

        write_disk_texture(&hash, &bytes);
        let texture = decode_texture(ctx, user_id, Some(&hash), &bytes)?;
        self.textures.insert(
            user_id,
            CachedAvatar {
                hash: Some(hash),
                texture: texture.clone(),
            },
        );
        Some(texture)
    }

    pub fn clear(&mut self) {
        self.textures.clear();
        self.requested.clear();
    }
}

/// Center-crops to square, resizes to 128x128, and preserves full alpha
/// transparency as PNG when present (or crisp JPEG for opaque photos).
pub fn process_avatar_bytes(
    bytes: &[u8],
    filename_hint: &str,
) -> Result<(Vec<u8>, String, String), String> {
    let hint_lower = filename_hint.to_ascii_lowercase();
    if hint_lower.ends_with(".gif") {
        return Err("GIF avatars are not supported; choose a PNG, JPG, or WebP image".into());
    }

    if let Ok(format) = image::guess_format(bytes) {
        if format == ImageFormat::Gif {
            return Err("GIF avatars are not supported; choose a still image".into());
        }
    }
    if is_animated_webp(bytes) {
        return Err("animated WebP avatars are not supported; choose a still image".into());
    }

    let decoded = image::load_from_memory(bytes)
        .map_err(|err| format!("could not decode '{filename_hint}': {err}"))?;
    let side = decoded.width().min(decoded.height()).max(1);
    let cropped = decoded.crop_imm(
        (decoded.width() - side) / 2,
        (decoded.height() - side) / 2,
        side,
        side,
    );
    let rgba = cropped
        .resize_exact(AVATAR_SIZE, AVATAR_SIZE, FilterType::Lanczos3)
        .to_rgba8();
    let has_transparency = rgba.pixels().any(|px| px[3] < 255);

    if has_transparency {
        let mut out = Vec::with_capacity(32 * 1024);
        for dim in [AVATAR_SIZE, 96, 64] {
            let candidate = if dim == AVATAR_SIZE {
                rgba.clone()
            } else {
                DynamicImage::ImageRgba8(rgba.clone())
                    .resize_exact(dim, dim, FilterType::Lanczos3)
                    .to_rgba8()
            };
            out.clear();
            PngEncoder::new_with_quality(&mut out, CompressionType::Best, PngFilter::Adaptive)
                .write_image(
                    candidate.as_raw(),
                    candidate.width(),
                    candidate.height(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(|err| format!("could not encode avatar as PNG: {err}"))?;
            if out.len() <= MAX_AVATAR_BYTES {
                let hash = avatar_hash(&out);
                return Ok((out, "image/png".to_string(), hash));
            }
        }
        return Err("transparent avatar is too large after compression; try a simpler image".into());
    }

    let rgb = DynamicImage::ImageRgba8(rgba).to_rgb8();
    let mut out = Vec::with_capacity(32 * 1024);
    let mut accepted = false;
    for quality in QUALITIES {
        out.clear();
        let mut encoder = JpegEncoder::new_with_quality(&mut out, quality);
        encoder
            .encode(
                rgb.as_raw(),
                AVATAR_SIZE,
                AVATAR_SIZE,
                ExtendedColorType::Rgb8,
            )
            .map_err(|err| format!("could not encode avatar as JPEG: {err}"))?;
        if out.len() <= MAX_AVATAR_BYTES {
            accepted = true;
            break;
        }
    }
    if !accepted {
        return Err("avatar is still too large after compression; try a simpler image".into());
    }

    let hash = avatar_hash(&out);
    Ok((out, "image/jpeg".to_string(), hash))
}

/// Opens a native file picker filtered to PNG/JPG/WebP. Returns `(bytes, filename)`.
pub fn pick_avatar_file() -> Option<(Vec<u8>, String)> {
    let path = rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
        .pick_file()?;
    let bytes = fs::read(&path).ok()?;
    let filename = path.file_name()?.to_string_lossy().into_owned();
    Some((bytes, filename))
}

pub fn avatar_hash(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn cache_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(CACHE_DIR);
    if let Err(err) = fs::create_dir_all(&dir) {
        tracing::warn!("cannot create avatar cache dir {CACHE_DIR}: {err}");
        return None;
    }
    Some(dir)
}

fn sanitize_hash(hash: &str) -> String {
    if !hash.is_empty() && hash.len() <= 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        hash.to_string()
    } else {
        avatar_hash(hash.as_bytes())
    }
}

fn disk_path(hash: &str) -> Option<PathBuf> {
    Some(cache_dir()?.join(format!("{}.img", sanitize_hash(hash))))
}

fn load_disk_texture(ctx: &egui::Context, user_id: i64, hash: &str) -> Option<TextureHandle> {
    let bytes = fs::read(disk_path(hash)?).ok()?;
    decode_texture(ctx, user_id, Some(hash), &bytes)
}

fn write_disk_texture(hash: &str, bytes: &[u8]) {
    if let Some(path) = disk_path(hash) {
        if let Err(err) = fs::write(path, bytes) {
            tracing::warn!("could not write avatar cache file for {hash}: {err}");
        }
    }
}

/// Decodes any supported avatar (PNG with transparency, JPEG, WebP) and applies a
/// smooth sub-pixel anti-aliased circular mask while preserving original PNG alpha.
fn decode_texture(
    ctx: &egui::Context,
    user_id: i64,
    hash: Option<&str>,
    bytes: &[u8],
) -> Option<TextureHandle> {
    let mut image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = (image.width(), image.height());
    let cx = (w as f32 - 1.0) * 0.5;
    let cy = (h as f32 - 1.0) * 0.5;
    let radius = (w.min(h) as f32) * 0.5 - 0.5;
    for (x, y, px) in image.enumerate_pixels_mut() {
        let dx = x as f32 - cx;
        let dy = y as f32 - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        let edge = (radius - dist + 0.5).clamp(0.0, 1.0);
        px[3] = ((px[3] as f32) * edge).round() as u8;
    }
    let color = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], image.as_raw());
    let name = match hash {
        Some(hash) => format!("avatar-{user_id}-{hash}"),
        None => format!("avatar-{user_id}"),
    };
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

fn spawn_fetch(api: crate::api::ApiClient, token: String, user_id: i64, tx: mpsc::Sender<UiEvent>) {
    let _ = std::thread::Builder::new()
        .name(format!("avatar-fetch-{user_id}"))
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    tracing::warn!("avatar fetch runtime failed: {err}");
                    return;
                }
            };
            runtime.block_on(async move {
                match api.get_avatar(&token, user_id).await {
                    Ok(info) => {
                        let _ = tx.send(UiEvent::AvatarLoaded {
                            user_id: info.user_id,
                            hash: info.hash,
                            mime: info.mime,
                            data_base64: info.data_base64,
                        });
                    }
                    Err(err) => tracing::warn!("avatar fetch failed for {user_id}: {err}"),
                }
            });
        });
}

fn is_animated_webp(bytes: &[u8]) -> bool {
    if bytes.len() < 16 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return false;
    }
    let mut offset = 12usize;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        if id == b"ANIM" || id == b"ANMF" {
            return true;
        }
        match offset.checked_add(8 + size + (size & 1)) {
            Some(next) if next > offset && next <= bytes.len() => offset = next,
            _ => break,
        }
    }
    bytes.windows(4).any(|window| window == b"ANIM")
}

fn prune_disk_cache(max_files: usize) {
    let Ok(entries) = fs::read_dir(CACHE_DIR) else {
        return;
    };
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            let modified = meta
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            Some((e.path(), modified))
        })
        .collect();
    if files.len() <= max_files {
        return;
    }
    files.sort_by_key(|(_, mtime)| *mtime);
    let remove_count = files.len() - max_files;
    for (path, _) in files.into_iter().take(remove_count) {
        let _ = fs::remove_file(path);
    }
}
