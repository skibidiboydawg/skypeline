//! Server-relayed screen sharing. Captures the primary monitor, scales it to
//! at most 1280x720, JPEG-encodes it and sends it through the WebSocket relay.

use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use skypeline_shared::WsClientMsg;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc as tmpsc;
use tokio::task::JoinHandle;

const MAX_WIDTH: u32 = 1280;
const MAX_HEIGHT: u32 = 720;
const FRAME_INTERVAL: Duration = Duration::from_millis(33); // ~30 fps

pub struct ScreenShare {
    stop: Arc<AtomicBool>,
    task: JoinHandle<()>,
}

impl ScreenShare {
    pub fn start(room: String, ws: tmpsc::UnboundedSender<WsClientMsg>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let task_stop = Arc::clone(&stop);
        let task = tokio::task::spawn_blocking(move || {
            let _ = ws.send(WsClientMsg::SetScreenShare {
                room: room.clone(),
                active: true,
            });

            while !task_stop.load(Ordering::Relaxed) {
                match capture_frame() {
                    Some((jpeg, width, height)) => {
                        let opus_b64 = base64::engine::general_purpose::STANDARD.encode(jpeg);
                        let _ = ws.send(WsClientMsg::ScreenFrame {
                            room: room.clone(),
                            width,
                            height,
                            jpeg_b64: opus_b64,
                        });
                    }
                    None => tracing::warn!("screen capture unavailable"),
                }
                std::thread::sleep(FRAME_INTERVAL);
            }

            let _ = ws.send(WsClientMsg::SetScreenShare {
                room,
                active: false,
            });
        });
        Self { stop, task }
    }

    pub fn stop(self) {
        self.stop.store(true, Ordering::Relaxed);
        self.task.abort();
    }
}

fn capture_frame() -> Option<(Vec<u8>, u32, u32)> {
    let monitors = xcap::Monitor::all().ok()?;
    let monitor = monitors
        .iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| monitors.first())?;
    let image = monitor.capture_image().ok()?;

    let (width, height) = fit(image.width(), image.height(), MAX_WIDTH, MAX_HEIGHT);
    let resized = image::imageops::resize(&image, width, height, FilterType::Triangle);

    let mut jpeg = Vec::with_capacity((width * height / 4) as usize);
    {
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg, 62);
        encoder
            .encode(
                resized.as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgba8,
            )
            .ok()?;
    }
    Some((jpeg, width, height))
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
