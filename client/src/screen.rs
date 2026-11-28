//! High-framerate server-relayed screen sharing with live FPS (15 / 30 / 60) and
//! resolution (480p / 720p / 1080p) settings.
//!
//! Optimized for low-latency smooth streaming:
//! - Caches the `xcap::Monitor` handle instead of re-enumerating displays per frame.
//! - Uses a single-pass fast downscale + RGBA->RGB split (skips resize completely when
//!   source already fits target resolution).
//! - Uses `Instant`-based deadline pacing instead of post-work sleeps so 30/60 FPS
//!   actually hits target frame rates.

use crate::events::UiEvent;
use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use skypeline_shared::WsClientMsg;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio::sync::mpsc as tmpsc;

const MAX_SOURCE_WIDTH: u32 = 7680;
const MAX_SOURCE_HEIGHT: u32 = 4320;
const MAX_OUTPUT_PIXELS: u64 = 1920 * 1080;
const STOP_POLL: Duration = Duration::from_millis(4);

pub struct ScreenShare {
    stop: Arc<AtomicBool>,
    /// Dropping a `JoinHandle` detaches the thread. The thread checks `stop`
    /// and sends the `SetScreenShare { active: false }` message on its way out.
    thread: Option<JoinHandle<()>>,
}

impl ScreenShare {
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        room: String,
        local_user_id: i64,
        local_username: String,
        fps: Arc<AtomicU32>,
        max_height: Arc<AtomicU32>,
        ws: tmpsc::UnboundedSender<WsClientMsg>,
        frames: tmpsc::Sender<WsClientMsg>,
        events: mpsc::Sender<UiEvent>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let task_stop = Arc::clone(&stop);

        let thread = thread::spawn(move || {
            let _ = ws.send(WsClientMsg::SetScreenShare {
                room: room.clone(),
                active: true,
            });

            let mut cached_monitor = acquire_primary_monitor();
            let mut failure_reported = false;
            let mut next_frame = Instant::now();
            let mut rgb_buf: Vec<u8> = Vec::new();
            let mut jpeg_buf: Vec<u8> = Vec::new();

            while !task_stop.load(Ordering::Acquire) {
                let target_fps = match fps.load(Ordering::Relaxed) {
                    15 => 15u32,
                    60 => 60u32,
                    _ => 30u32,
                };
                let target_h = match max_height.load(Ordering::Relaxed) {
                    480 => 480u32,
                    720 => 720u32,
                    _ => 1080u32,
                };
                let target_w = ((target_h * 16) / 9).min(1920);
                let quality = match target_fps {
                    15 => 72u8,
                    30 => 60u8,
                    _ => 50u8,
                };
                let interval = Duration::from_micros(1_000_000 / u64::from(target_fps));

                if cached_monitor.is_none() {
                    cached_monitor = acquire_primary_monitor();
                }

                let captured = if let Some(ref monitor) = cached_monitor {
                    catch_unwind(AssertUnwindSafe(|| {
                        capture_frame(monitor, target_w, target_h, quality, &mut rgb_buf, &mut jpeg_buf)
                    }))
                } else {
                    Ok(None)
                };

                match captured {
                    Ok(Some((rgba, width, height))) => {
                        failure_reported = false;
                        // Immediate live local preview
                        let _ = events.send(UiEvent::ScreenFrame {
                            from: local_user_id,
                            username: local_username.clone(),
                            width,
                            height,
                            rgba,
                        });
                        // Only base64-encode & send over relay when the bounded channel has space
                        // so we never waste CPU encoding frames that would be dropped.
                        if frames.capacity() > 0 {
                            let jpeg_b64 =
                                base64::engine::general_purpose::STANDARD.encode(&jpeg_buf);
                            let _ = frames.try_send(WsClientMsg::ScreenFrame {
                                room: room.clone(),
                                width,
                                height,
                                jpeg_b64,
                            });
                        }
                    }
                    Ok(None) | Err(_) => {
                        cached_monitor = None;
                        if !failure_reported {
                            failure_reported = true;
                            let _ = events.send(UiEvent::Error(
                                "Screen capture is unavailable on this display".into(),
                            ));
                        }
                    }
                }

                // Deadline-based frame pacing so 15 / 30 / 60 FPS actually hits target cadence
                next_frame += interval;
                let now = Instant::now();
                if next_frame + interval * 2 < now {
                    next_frame = now;
                }
                while Instant::now() < next_frame {
                    if task_stop.load(Ordering::Acquire) {
                        break;
                    }
                    let rem = next_frame.saturating_duration_since(Instant::now());
                    thread::sleep(rem.min(STOP_POLL));
                }
            }

            let _ = ws.send(WsClientMsg::SetScreenShare {
                room,
                active: false,
            });
            let _ = events.send(UiEvent::ScreenStopped {
                from: local_user_id,
            });
        });

        Self {
            stop,
            thread: Some(thread),
        }
    }

    pub fn stop(self) {
        self.stop.store(true, Ordering::Release);
        drop(self.thread);
    }
}

fn acquire_primary_monitor() -> Option<xcap::Monitor> {
    let monitors = xcap::Monitor::all().ok()?;
    let primary = monitors
        .iter()
        .position(|m| m.is_primary().unwrap_or(false))
        .unwrap_or(0);
    monitors.into_iter().nth(primary)
}

fn capture_frame(
    monitor: &xcap::Monitor,
    max_w: u32,
    max_h: u32,
    quality: u8,
    rgb_buf: &mut Vec<u8>,
    jpeg_buf: &mut Vec<u8>,
) -> Option<(Vec<u8>, u32, u32)> {
    let image = monitor.capture_image().ok()?;
    let src_w = image.width();
    let src_h = image.height();
    if src_w == 0 || src_h == 0 || src_w > MAX_SOURCE_WIDTH || src_h > MAX_SOURCE_HEIGHT {
        return None;
    }

    let (dst_w, dst_h) = fit(src_w, src_h, max_w, max_h);
    if dst_w == 0 || dst_h == 0 || (dst_w as u64) * (dst_h as u64) > MAX_OUTPUT_PIXELS {
        return None;
    }

    let src_raw = image.as_raw();
    let rgba = fast_scale_and_rgb(src_raw, src_w, src_h, dst_w, dst_h, rgb_buf);

    jpeg_buf.clear();
    jpeg_buf.reserve((dst_w * dst_h / 5) as usize);
    {
        let mut encoder = JpegEncoder::new_with_quality(&mut *jpeg_buf, quality);
        encoder
            .encode(rgb_buf, dst_w, dst_h, image::ExtendedColorType::Rgb8)
            .ok()?;
    }
    if jpeg_buf.is_empty() {
        return None;
    }
    Some((rgba, dst_w, dst_h))
}

/// Fast single-pass nearest/stride scaler that simultaneously produces RGBA (for local GPU preview)
/// and packed RGB (for JPEG encoding). Skips resizing completely when dimensions already match.
fn fast_scale_and_rgb(
    src_rgba: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    rgb_out: &mut Vec<u8>,
) -> Vec<u8> {
    let pixel_count = (dst_w as usize) * (dst_h as usize);
    rgb_out.clear();
    rgb_out.reserve(pixel_count * 3);

    if src_w == dst_w && src_h == dst_h {
        for px in src_rgba.chunks_exact(4) {
            rgb_out.extend_from_slice(&px[..3]);
        }
        return src_rgba.to_vec();
    }

    let mut dst_rgba = Vec::with_capacity(pixel_count * 4);
    let x_map: Vec<usize> = (0..dst_w)
        .map(|x| ((x as u64 * src_w as u64) / dst_w as u64) as usize * 4)
        .collect();
    let src_stride = (src_w as usize) * 4;

    for y in 0..dst_h {
        let sy = ((y as u64 * src_h as u64) / dst_h as u64) as usize;
        let row_start = sy * src_stride;
        for &sx_off in &x_map {
            let idx = row_start + sx_off;
            let r = src_rgba[idx];
            let g = src_rgba[idx + 1];
            let b = src_rgba[idx + 2];
            let a = src_rgba[idx + 3];
            dst_rgba.extend_from_slice(&[r, g, b, a]);
            rgb_out.extend_from_slice(&[r, g, b]);
        }
    }
    dst_rgba
}

pub fn fit(width: u32, height: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (max_w, max_h);
    }
    let scale = (max_w as f32 / width as f32).min(max_h as f32 / height as f32);
    if scale >= 1.0 {
        (width, height)
    } else {
        (
            (width as f32 * scale).round().max(1.0) as u32,
            (height as f32 * scale).round().max(1.0) as u32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_rgba_capture_as_rgb_jpeg() {
        let src = vec![200u8, 40, 20, 255].repeat(32 * 18);
        let (w, h) = fit(32, 18, 1920, 1080);
        let mut rgb = Vec::new();
        let rgba = fast_scale_and_rgb(&src, 32, 18, w, h, &mut rgb);
        assert_eq!(rgba.len(), (w * h * 4) as usize);
        assert_eq!(rgb.len(), (w * h * 3) as usize);

        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, 60)
            .encode(&rgb, w, h, image::ExtendedColorType::Rgb8)
            .expect("RGB JPEG encoding should succeed");
        assert!(!jpeg.is_empty());
    }

    #[test]
    fn downscales_1440p_to_1080p_and_720p() {
        assert_eq!(fit(2560, 1440, 1920, 1080), (1920, 1080));
        assert_eq!(fit(1920, 1080, 1280, 720), (1280, 720));
        assert_eq!(fit(1920, 1080, 1920, 1080), (1920, 1080));
    }
}
