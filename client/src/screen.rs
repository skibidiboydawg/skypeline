//! Server-relayed screen sharing. Captures the primary monitor, scales it to
//! at most 1280x720, JPEG-encodes it and sends it through the WebSocket relay.
//!
//! Capture runs on a dedicated OS thread: a panic in the capture/encode stack
//! is caught and reported as a UI notice instead of taking down the app. Frames
//! go through a small bounded channel so a slow relay drops old frames instead
//! of letting memory grow without limit.

use crate::events::UiEvent;
use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use skypeline_shared::WsClientMsg;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tokio::sync::mpsc as tmpsc;

const MAX_WIDTH: u32 = 1280;
const MAX_HEIGHT: u32 = 720;
const MAX_SOURCE_WIDTH: u32 = 7680;
const MAX_SOURCE_HEIGHT: u32 = 4320;
const FRAME_INTERVAL: Duration = Duration::from_millis(66); // ~15 fps
const STOP_POLL: Duration = Duration::from_millis(10);

pub struct ScreenShare {
    stop: Arc<AtomicBool>,
    /// Dropping a `JoinHandle` detaches the thread. The thread checks `stop`
    /// and sends the `SetScreenShare { active: false }` message on its way out.
    thread: Option<JoinHandle<()>>,
}

impl ScreenShare {
    pub fn start(
        room: String,
        local_user_id: i64,
        local_username: String,
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

            let mut failure_reported = false;
            while !task_stop.load(Ordering::Acquire) {
                let captured = catch_unwind(AssertUnwindSafe(capture_frame));
                match captured {
                    Ok(Some((jpeg, rgba, width, height))) => {
                        // Show live local preview immediately in our own call stage
                        let _ = events.send(UiEvent::ScreenFrame {
                            from: local_user_id,
                            username: local_username.clone(),
                            width,
                            height,
                            rgba,
                        });
                        let jpeg_b64 = base64::engine::general_purpose::STANDARD.encode(jpeg);
                        // Drop the newest frame if the relay is behind. This
                        // keeps screen share best-effort without growing the
                        // outbound queue without bound.
                        let _ = frames.try_send(WsClientMsg::ScreenFrame {
                            room: room.clone(),
                            width,
                            height,
                            jpeg_b64,
                        });
                    }
                    Ok(None) | Err(_) => {
                        if !failure_reported {
                            failure_reported = true;
                            let _ = events.send(UiEvent::Error(
                                "Screen capture is unavailable on this display".into(),
                            ));
                        }
                    }
                }

                let mut waited = Duration::ZERO;
                while waited < FRAME_INTERVAL {
                    if task_stop.load(Ordering::Acquire) {
                        break;
                    }
                    thread::sleep(STOP_POLL);
                    waited += STOP_POLL;
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
        // Detach rather than join: joining would freeze the UI for the
        // duration of an in-flight capture/encode.
        drop(self.thread);
    }
}

fn capture_frame() -> Option<(Vec<u8>, Vec<u8>, u32, u32)> {
    let monitors = xcap::Monitor::all().ok()?;
    let monitor = monitors
        .iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| monitors.first())?;
    let image = monitor.capture_image().ok()?;

    let source_w = image.width();
    let source_h = image.height();
    if source_w == 0 || source_h == 0 || source_w > MAX_SOURCE_WIDTH || source_h > MAX_SOURCE_HEIGHT
    {
        return None;
    }

    let (width, height) = fit(source_w, source_h, MAX_WIDTH, MAX_HEIGHT);
    if width == 0 || height == 0 || (width as u64) * (height as u64) > 1_000_000 {
        return None;
    }

    let resized = image::imageops::resize(&image, width, height, FilterType::Triangle);
    let rgba = resized.as_raw().clone();
    // JPEG does not support RGBA. Hand the encoder an RGB image instead of
    // silently failing on every frame.
    let rgb = image::DynamicImage::ImageRgba8(resized).to_rgb8();

    let mut jpeg = Vec::with_capacity((width * height / 4) as usize);
    {
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg, 62);
        encoder
            .encode(rgb.as_raw(), width, height, image::ExtendedColorType::Rgb8)
            .ok()?;
    }
    if jpeg.is_empty() {
        return None;
    }
    Some((jpeg, rgba, width, height))
}

fn fit(width: u32, height: u32, max_w: u32, max_h: u32) -> (u32, u32) {
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
        let image = image::RgbaImage::from_pixel(32, 18, image::Rgba([200, 40, 20, 255]));
        let (width, height) = fit(image.width(), image.height(), MAX_WIDTH, MAX_HEIGHT);
        let resized = image::imageops::resize(&image, width, height, FilterType::Triangle);
        let rgb = image::DynamicImage::ImageRgba8(resized).to_rgb8();
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, 62)
            .encode(rgb.as_raw(), width, height, image::ExtendedColorType::Rgb8)
            .expect("RGB JPEG encoding should succeed");
        assert!(!jpeg.is_empty());
    }
}
