use crate::auth::{bearer, internal, ApiError};
use crate::{db, hub, AppState};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder};
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageFormat};
use skypeline_shared::{AvatarInfo, AvatarUpload, WsServerMsg};
use sqlx::Row;
use std::sync::Arc;

const MAX_DECODED_BYTES: usize = 8 * 1024 * 1024;
const MAX_ENCODED_BYTES: usize = (MAX_DECODED_BYTES / 3 + 1) * 4;
const AVATAR_PIXELS: u32 = 128;
const MAX_AVATAR_BYTES: usize = 96 * 1024;

fn bad_request(message: &str) -> ApiError {
    (StatusCode::BAD_REQUEST, message.to_string())
}

/// Animated WebP files carry an `ANIM`/`ANMF` chunk (or the animation flag in
/// `VP8X`); `image` would otherwise silently decode just the first frame.
fn is_animated_webp(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return false;
    }
    let mut offset = 12usize;
    while offset + 8 <= bytes.len() {
        let fourcc = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let payload = offset + 8;
        if fourcc == b"ANIM" || fourcc == b"ANMF" {
            return true;
        }
        if fourcc == b"VP8X" && payload < bytes.len() && bytes[payload] & 0x02 != 0 {
            return true;
        }
        let Some(next) = payload
            .checked_add(size)
            .and_then(|end| end.checked_add(size % 2))
        else {
            return false;
        };
        if next <= payload || next > bytes.len() {
            return false;
        }
        offset = next;
    }
    false
}

/// `POST /api/avatar` — normalize an uploaded image to a small JPEG avatar.
pub async fn upload(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<AvatarUpload>,
) -> Result<Json<AvatarInfo>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;

    let format = match req.mime.as_str() {
        "image/jpeg" => ImageFormat::Jpeg,
        "image/png" => ImageFormat::Png,
        "image/webp" => ImageFormat::WebP,
        _ => return Err(bad_request("unsupported avatar mime type")),
    };
    if req.hash.is_empty()
        || req.hash.len() > 64
        || !req
            .hash
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-' || c == '_')
    {
        return Err(bad_request("invalid avatar hash"));
    }
    if req.data_base64.len() > MAX_ENCODED_BYTES {
        return Err(bad_request("avatar image is too large"));
    }
    let bytes = STANDARD
        .decode(req.data_base64.as_bytes())
        .map_err(|_| bad_request("invalid base64 image data"))?;
    if bytes.is_empty() || bytes.len() > MAX_DECODED_BYTES {
        return Err(bad_request("avatar image is too large"));
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Err(bad_request("animated images are not supported"));
    }
    if is_animated_webp(&bytes) {
        return Err(bad_request("animated images are not supported"));
    }

    let image = image::load_from_memory_with_format(&bytes, format)
        .map_err(|_| bad_request("invalid image data"))?;
    // Center-crop to square so avatars never stretch or distort.
    let side = image.width().min(image.height()).max(1);
    let cropped = image.crop_imm((image.width() - side) / 2, (image.height() - side) / 2, side, side);
    let rgba = cropped
        .resize_exact(AVATAR_PIXELS, AVATAR_PIXELS, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let has_transparency = rgba.pixels().any(|px| px[3] < 255);

    let (encoded, out_mime): (Vec<u8>, &'static str) = if has_transparency {
        // Preserve full alpha transparency using compressed PNG!
        let mut png_bytes = Vec::new();
        for dim in [AVATAR_PIXELS, 96, 64] {
            let candidate = if dim == AVATAR_PIXELS {
                rgba.clone()
            } else {
                DynamicImage::ImageRgba8(rgba.clone())
                    .resize_exact(dim, dim, image::imageops::FilterType::Lanczos3)
                    .to_rgba8()
            };
            png_bytes.clear();
            PngEncoder::new_with_quality(&mut png_bytes, CompressionType::Best, PngFilter::Adaptive)
                .write_image(
                    candidate.as_raw(),
                    candidate.width(),
                    candidate.height(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(|e| bad_request(&format!("failed to encode PNG avatar: {e}")))?;
            if png_bytes.len() <= MAX_AVATAR_BYTES {
                break;
            }
        }
        if png_bytes.is_empty() || png_bytes.len() > MAX_AVATAR_BYTES {
            return Err(bad_request("avatar is too large after PNG compression"));
        }
        (png_bytes, "image/png")
    } else {
        let rgb = DynamicImage::ImageRgba8(rgba).to_rgb8();
        let mut out_opt = None;
        for quality in [82u8, 72, 62, 52, 42] {
            let mut out = Vec::new();
            JpegEncoder::new_with_quality(&mut out, quality)
                .write_image(rgb.as_raw(), rgb.width(), rgb.height(), ExtendedColorType::Rgb8)
                .map_err(|e| bad_request(&format!("failed to encode avatar: {e}")))?;
            if out.len() <= MAX_AVATAR_BYTES {
                out_opt = Some(out);
                break;
            }
        }
        let out = out_opt.ok_or_else(|| bad_request("avatar is too large after compression"))?;
        (out, "image/jpeg")
    };

    sqlx::query(
        "UPDATE users SET avatar_data = $1, avatar_mime = $2, avatar_hash = $3
         WHERE id = $4",
    )
    .bind(&encoded)
    .bind(out_mime)
    .bind(&req.hash)
    .bind(claims.sub)
    .execute(&state.db)
    .await
    .map_err(internal)?;

    // Notify friends so their cached avatar hash refreshes right away.
    if let Ok(friends) = db::accepted_friend_ids(&state.db, claims.sub).await {
        if !friends.is_empty() {
            state.hub.send(
                hub::Scope::Users(friends),
                None,
                &WsServerMsg::FriendsUpdated,
            );
        }
    }

    Ok(Json(AvatarInfo {
        user_id: claims.sub,
        hash: req.hash,
        mime: out_mime.to_string(),
        data_base64: STANDARD.encode(&encoded),
    }))
}

/// `GET /api/avatar/{user_id}` — authenticated avatar fetch; never public.
pub async fn get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
) -> Result<Json<AvatarInfo>, ApiError> {
    bearer(&headers, &state.jwt_secret)?;
    let row = sqlx::query("SELECT avatar_hash, avatar_mime, avatar_data FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(internal)?
        .ok_or((StatusCode::NOT_FOUND, "user not found".to_string()))?;

    let hash: Option<String> = row.get("avatar_hash");
    let mime: Option<String> = row.get("avatar_mime");
    let data: Option<Vec<u8>> = row.get("avatar_data");
    match (hash, mime, data) {
        (Some(hash), Some(mime), Some(data)) => Ok(Json(AvatarInfo {
            user_id,
            hash,
            mime,
            data_base64: STANDARD.encode(&data),
        })),
        _ => Err((StatusCode::NOT_FOUND, "avatar not found".to_string())),
    }
}
