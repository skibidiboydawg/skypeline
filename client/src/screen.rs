//! High-framerate (15 / 30 / 60 FPS, up to 1080p) screen sharing with hardware-accelerated
//! DXGI Desktop Duplication on Windows, live OS mouse cursor compositing, 1ms timer precision,
//! and a decoupled 2-thread capture + 4:2:0 SIMD JPEG encode pipeline.

use crate::events::UiEvent;
use base64::Engine;
use jpeg_encoder::{ColorType, Encoder as JpegEncoder, SamplingFactor};
use skypeline_shared::WsClientMsg;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio::sync::mpsc as tmpsc;

const MAX_SOURCE_WIDTH: u32 = 7680;
const MAX_SOURCE_HEIGHT: u32 = 4320;
const MAX_OUTPUT_PIXELS: u64 = 1920 * 1080;
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(450);

struct PendingEncode {
    rgb: Vec<u8>,
    width: u32,
    height: u32,
    target_fps: u32,
    target_h: u32,
}

#[derive(Default)]
struct EncodeSlot {
    latest: Option<PendingEncode>,
    recycle: Option<Vec<u8>>,
    closed: bool,
}

pub struct ScreenShare {
    stop: Arc<AtomicBool>,
    slot: Arc<(Mutex<EncodeSlot>, Condvar)>,
    capture_thread: Option<JoinHandle<()>>,
    encode_thread: Option<JoinHandle<()>>,
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
        let slot = Arc::new((Mutex::new(EncodeSlot::default()), Condvar::new()));

        // Thread 2: Dedicated 4:2:0 JPEG encoder + adaptive quality WebSocket sender.
        // Always encodes the freshest frame from the 1-slot handoff (zero queue lag).
        let enc_stop = Arc::clone(&stop);
        let enc_slot = Arc::clone(&slot);
        let enc_room = room.clone();
        let encode_thread = thread::spawn(move || {
            let mut jpeg_buf: Vec<u8> = Vec::with_capacity(96 * 1024);
            let mut quality_offset: i32 = 0;

            loop {
                let job = {
                    let (lock, cv) = &*enc_slot;
                    let mut guard = match lock.lock() {
                        Ok(g) => g,
                        Err(_) => break,
                    };
                    while guard.latest.is_none() && !guard.closed && !enc_stop.load(Ordering::Acquire) {
                        guard = match cv.wait(guard) {
                            Ok(g) => g,
                            Err(_) => return,
                        };
                    }
                    if guard.closed || enc_stop.load(Ordering::Acquire) {
                        break;
                    }
                    guard.latest.take()
                };

                let Some(mut job) = job else { continue };

                // If the outbound WebSocket frame queue is saturated, skip encoding this frame
                // and nudge quality down slightly so subsequent frames fly through faster.
                if frames.capacity() == 0 {
                    quality_offset = (quality_offset - 4).max(-22);
                    let (lock, _) = &*enc_slot;
                    if let Ok(mut guard) = lock.lock() {
                        guard.recycle = Some(job.rgb);
                    }
                    continue;
                }

                let base_q = base_jpeg_quality(job.target_fps, job.target_h);
                let quality = (base_q as i32 + quality_offset).clamp(28, 82) as u8;

                jpeg_buf.clear();
                let encoded_ok = encode_rgb_jpeg_420(&job.rgb, job.width, job.height, quality, &mut jpeg_buf);

                // Return the RGB buffer to the capture thread for reuse.
                job.rgb.clear();
                {
                    let (lock, _) = &*enc_slot;
                    if let Ok(mut guard) = lock.lock() {
                        guard.recycle = Some(job.rgb);
                    }
                }

                if encoded_ok && !jpeg_buf.is_empty() {
                    let jpeg_b64 = base64::engine::general_purpose::STANDARD.encode(&jpeg_buf);
                    match frames.try_send(WsClientMsg::ScreenFrame {
                        room: enc_room.clone(),
                        width: job.width,
                        height: job.height,
                        jpeg_b64,
                    }) {
                        Ok(()) => {
                            // Gently recover visual quality whenever the network keeps up.
                            if quality_offset < 0 {
                                quality_offset = (quality_offset + 1).min(0);
                            }
                        }
                        Err(_) => {
                            quality_offset = (quality_offset - 4).max(-22);
                        }
                    }
                }
            }
        });

        // Thread 1: High-speed hardware capture + cursor compositing + local preview.
        let cap_stop = Arc::clone(&stop);
        let cap_slot = Arc::clone(&slot);
        let capture_thread = thread::spawn(move || {
            #[cfg(target_os = "windows")]
            let _timer_guard = win_capture::TimerPeriodGuard::new(1);

            let _ = ws.send(WsClientMsg::SetScreenShare {
                room: room.clone(),
                active: true,
            });

            let mut capturer = PlatformCapturer::new();
            let mut failure_reported = false;
            let mut next_frame = Instant::now();
            let mut last_sent_at = Instant::now() - HEARTBEAT_INTERVAL;
            let mut last_cursor_sig: (usize, i32, i32) = (0, i32::MIN, i32::MIN);
            let mut last_dims: (u32, u32) = (0, 0);
            let mut rgb_scratch: Vec<u8> = Vec::new();

            while !cap_stop.load(Ordering::Acquire) {
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
                let interval = Duration::from_micros(1_000_000 / u64::from(target_fps));

                // Grab a recycled RGB buffer from the encoder thread if available.
                {
                    let (lock, _) = &*cap_slot;
                    if let Ok(mut guard) = lock.lock() {
                        if let Some(buf) = guard.recycle.take() {
                            rgb_scratch = buf;
                        }
                    }
                }

                let captured = catch_unwind(AssertUnwindSafe(|| {
                    capturer.capture(target_w, target_h, &mut rgb_scratch)
                }));

                match captured {
                    Ok(Some(frame)) => {
                        failure_reported = false;
                        let cursor_moved = frame.cursor_sig != last_cursor_sig;
                        last_cursor_sig = frame.cursor_sig;
                        let dims_changed = (frame.width, frame.height) != last_dims;
                        last_dims = (frame.width, frame.height);

                        let now = Instant::now();
                        let should_transmit = frame.pixels_changed
                            || cursor_moved
                            || dims_changed
                            || now.duration_since(last_sent_at) >= HEARTBEAT_INTERVAL;

                        // Instant local preview at full capture cadence.
                        if should_transmit {
                            let _ = events.send(UiEvent::ScreenFrame {
                                from: local_user_id,
                                username: local_username.clone(),
                                width: frame.width,
                                height: frame.height,
                                rgba: frame.rgba,
                            });
                        }

                        if should_transmit {
                            last_sent_at = now;
                            let rgb_to_encode = std::mem::take(&mut rgb_scratch);
                            let (lock, cv) = &*cap_slot;
                            if let Ok(mut guard) = lock.lock() {
                                if let Some(stale) = guard.latest.replace(PendingEncode {
                                    rgb: rgb_to_encode,
                                    width: frame.width,
                                    height: frame.height,
                                    target_fps,
                                    target_h,
                                }) {
                                    // Reuse the evicted stale buffer immediately.
                                    rgb_scratch = stale.rgb;
                                }
                                cv.notify_one();
                            }
                        }
                    }
                    Ok(None) | Err(_) => {
                        capturer.reset();
                        if !failure_reported {
                            failure_reported = true;
                            let _ = events.send(UiEvent::Error(
                                "Screen capture is unavailable on this display".into(),
                            ));
                        }
                        thread::sleep(Duration::from_millis(80));
                    }
                }

                // Precision deadline pacing: sleep until 1.5ms before deadline, then spin-wait
                // for sub-millisecond frame timing accuracy so 60 FPS hits 60.0 FPS.
                next_frame += interval;
                let now = Instant::now();
                if next_frame + interval < now {
                    next_frame = now;
                }
                while !cap_stop.load(Ordering::Acquire) {
                    let now = Instant::now();
                    if now >= next_frame {
                        break;
                    }
                    let rem = next_frame.saturating_duration_since(now);
                    if rem > Duration::from_millis(2) {
                        thread::sleep((rem - Duration::from_millis(1)).min(Duration::from_millis(4)));
                    } else {
                        thread::yield_now();
                    }
                }
            }

            {
                let (lock, cv) = &*cap_slot;
                if let Ok(mut guard) = lock.lock() {
                    guard.closed = true;
                    cv.notify_all();
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
            slot,
            capture_thread: Some(capture_thread),
            encode_thread: Some(encode_thread),
        }
    }

    pub fn stop(self) {
        self.stop.store(true, Ordering::Release);
        let (lock, cv) = &*self.slot;
        if let Ok(mut guard) = lock.lock() {
            guard.closed = true;
            cv.notify_all();
        }
        drop(self.capture_thread);
        drop(self.encode_thread);
    }
}

fn base_jpeg_quality(fps: u32, height: u32) -> u8 {
    match (fps, height) {
        (60, 1080) => 42,
        (60, 720) => 50,
        (60, _) => 58,
        (30, 1080) => 52,
        (30, 720) => 60,
        (30, _) => 66,
        (_, 1080) => 62,
        (_, 720) => 70,
        _ => 76,
    }
}

fn encode_rgb_jpeg_420(rgb: &[u8], width: u32, height: u32, quality: u8, out: &mut Vec<u8>) -> bool {
    let Ok(w) = u16::try_from(width) else {
        return false;
    };
    let Ok(h) = u16::try_from(height) else {
        return false;
    };
    out.clear();
    let mut enc = JpegEncoder::new(out, quality);
    enc.set_sampling_factor(SamplingFactor::F_2_2);
    enc.encode(rgb, w, h, ColorType::Rgb).is_ok()
}

struct CapturedOutput {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    pixels_changed: bool,
    cursor_sig: (usize, i32, i32),
}

/// Fast single-pass scaler converting BGRA (DXGI / Windows DIB native order) directly into
/// RGBA (for GPU preview) and packed RGB (for 4:2:0 JPEG encoding).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn fast_bgra_to_rgba_and_rgb(
    src_bgra: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    rgb_out: &mut Vec<u8>,
) -> Vec<u8> {
    let pixel_count = (dst_w as usize) * (dst_h as usize);
    rgb_out.clear();
    rgb_out.reserve(pixel_count * 3);
    let mut dst_rgba = Vec::with_capacity(pixel_count * 4);

    if src_w == dst_w && src_h == dst_h {
        for px in src_bgra.chunks_exact(4) {
            let b = px[0];
            let g = px[1];
            let r = px[2];
            dst_rgba.extend_from_slice(&[r, g, b, 255]);
            rgb_out.extend_from_slice(&[r, g, b]);
        }
        return dst_rgba;
    }

    let x_map: Vec<usize> = (0..dst_w)
        .map(|x| ((x as u64 * src_w as u64) / dst_w as u64) as usize * 4)
        .collect();
    let src_stride = (src_w as usize) * 4;

    for y in 0..dst_h {
        let sy = ((y as u64 * src_h as u64) / dst_h as u64) as usize;
        let row_start = sy * src_stride;
        for &sx_off in &x_map {
            let idx = row_start + sx_off;
            let b = src_bgra[idx];
            let g = src_bgra[idx + 1];
            let r = src_bgra[idx + 2];
            dst_rgba.extend_from_slice(&[r, g, b, 255]);
            rgb_out.extend_from_slice(&[r, g, b]);
        }
    }
    dst_rgba
}

/// Fast single-pass scaler converting RGBA (xcap fallback) into RGBA + packed RGB.
#[cfg_attr(target_os = "windows", allow(dead_code))]
fn fast_rgba_to_rgba_and_rgb(
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

// ============================================================================
// Windows Native DXGI Desktop Duplication + Persistent GDI Fallback + OS Cursor
// ============================================================================
#[cfg(target_os = "windows")]
mod win_capture {
    use super::{
        fast_bgra_to_rgba_and_rgb, fit, CapturedOutput, MAX_OUTPUT_PIXELS, MAX_SOURCE_HEIGHT,
        MAX_SOURCE_WIDTH,
    };
    use std::ffi::c_void;
    use std::mem;
    use std::ptr;
    use std::slice;
    use std::time::{Duration, Instant};
    use windows::core::Interface;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
    use windows::Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D,
        D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE,
        D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    };
    use windows::Win32::Graphics::Dxgi::{
        IDXGIDevice, IDXGIOutput1, IDXGIOutputDuplication, IDXGIResource,
        DXGI_ERROR_WAIT_TIMEOUT, DXGI_OUTDUPL_FRAME_INFO,
    };
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
        SRCCOPY,
    };
    use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
    use windows::Win32::UI::WindowsAndMessaging::{
        DrawIconEx, GetCursorInfo, GetIconInfo, GetSystemMetrics, CURSORINFO, CURSOR_SHOWING,
        DI_NORMAL, HCURSOR, HICON, ICONINFO, SM_CXCURSOR, SM_CXSCREEN, SM_CYCURSOR, SM_CYSCREEN,
    };

    pub struct TimerPeriodGuard(u32);

    impl TimerPeriodGuard {
        pub fn new(ms: u32) -> Self {
            unsafe {
                let _ = timeBeginPeriod(ms);
            }
            Self(ms)
        }
    }

    impl Drop for TimerPeriodGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = timeEndPeriod(self.0);
            }
        }
    }

    /// Cached alpha-recovered cursor pixel (handles normal alpha, drop shadows, AND XOR I-beams).
    #[derive(Clone, Copy)]
    struct CursorPixel {
        r: u8,
        g: u8,
        b: u8,
        alpha: u8,
        invert: bool,
    }

    struct CursorSprite {
        width: u32,
        height: u32,
        hot_x: i32,
        hot_y: i32,
        pixels: Vec<CursorPixel>,
    }

    struct CursorOverlay {
        last_handle: usize,
        sprite: Option<CursorSprite>,
    }

    impl CursorOverlay {
        fn new() -> Self {
            Self {
                last_handle: 0,
                sprite: None,
            }
        }

        /// Queries the live OS cursor and composites it onto both `dst_rgba` and `dst_rgb`.
        /// Returns `(cursor_handle, x, y)` signature for change detection.
        fn composite(
            &mut self,
            monitor_x: i32,
            monitor_y: i32,
            src_w: u32,
            src_h: u32,
            dst_w: u32,
            dst_h: u32,
            dst_rgba: &mut [u8],
            dst_rgb: &mut [u8],
        ) -> (usize, i32, i32) {
            unsafe {
                let mut ci = CURSORINFO {
                    cbSize: mem::size_of::<CURSORINFO>() as u32,
                    ..Default::default()
                };
                if GetCursorInfo(&mut ci).is_err() || (ci.flags.0 & CURSOR_SHOWING.0) == 0 {
                    return (0, i32::MIN, i32::MIN);
                }
                let hcursor = ci.hCursor;
                let handle_id = hcursor.0 as usize;
                if handle_id == 0 {
                    return (0, i32::MIN, i32::MIN);
                }
                if handle_id != self.last_handle || self.sprite.is_none() {
                    self.last_handle = handle_id;
                    self.sprite = extract_cursor_sprite(hcursor);
                }

                let sig = (handle_id, ci.ptScreenPos.x, ci.ptScreenPos.y);
                let Some(sprite) = self.sprite.as_ref() else {
                    return sig;
                };

                let rel_x = ci.ptScreenPos.x - monitor_x - sprite.hot_x;
                let rel_y = ci.ptScreenPos.y - monitor_y - sprite.hot_y;
                if rel_x + sprite.width as i32 <= 0
                    || rel_y + sprite.height as i32 <= 0
                    || rel_x >= src_w as i32
                    || rel_y >= src_h as i32
                {
                    return sig;
                }

                let scale_x = dst_w as f32 / src_w as f32;
                let scale_y = dst_h as f32 / src_h as f32;
                let out_x0 = (rel_x as f32 * scale_x).round() as i32;
                let out_y0 = (rel_y as f32 * scale_y).round() as i32;
                let out_cw = ((sprite.width as f32 * scale_x).round() as i32).max(1);
                let out_ch = ((sprite.height as f32 * scale_y).round() as i32).max(1);

                for oy in 0..out_ch {
                    let dy = out_y0 + oy;
                    if dy < 0 || dy >= dst_h as i32 {
                        continue;
                    }
                    let sy = ((oy as u64 * sprite.height as u64) / out_ch as u64)
                        .min(sprite.height as u64 - 1) as usize;
                    let src_row = sy * sprite.width as usize;
                    let dst_row = dy as usize * dst_w as usize;

                    for ox in 0..out_cw {
                        let dx = out_x0 + ox;
                        if dx < 0 || dx >= dst_w as i32 {
                            continue;
                        }
                        let sx = ((ox as u64 * sprite.width as u64) / out_cw as u64)
                            .min(sprite.width as u64 - 1) as usize;
                        let px = sprite.pixels[src_row + sx];
                        if !px.invert && px.alpha == 0 {
                            continue;
                        }

                        let p_idx = dst_row + dx as usize;
                        let i4 = p_idx * 4;
                        let i3 = p_idx * 3;

                        let (nr, ng, nb) = if px.invert {
                            (
                                255u8.saturating_sub(dst_rgba[i4]),
                                255u8.saturating_sub(dst_rgba[i4 + 1]),
                                255u8.saturating_sub(dst_rgba[i4 + 2]),
                            )
                        } else if px.alpha == 255 {
                            (px.r, px.g, px.b)
                        } else {
                            let inv = 255u16 - u16::from(px.alpha);
                            (
                                (u16::from(px.r) + (u16::from(dst_rgba[i4]) * inv) / 255).min(255)
                                    as u8,
                                (u16::from(px.g) + (u16::from(dst_rgba[i4 + 1]) * inv) / 255)
                                    .min(255) as u8,
                                (u16::from(px.b) + (u16::from(dst_rgba[i4 + 2]) * inv) / 255)
                                    .min(255) as u8,
                            )
                        };

                        dst_rgba[i4] = nr;
                        dst_rgba[i4 + 1] = ng;
                        dst_rgba[i4 + 2] = nb;
                        dst_rgb[i3] = nr;
                        dst_rgb[i3 + 1] = ng;
                        dst_rgb[i3 + 2] = nb;
                    }
                }

                sig
            }
        }
    }

    /// Extracts an exact RGBA + XOR cursor sprite via dual-pass black/white `DrawIconEx`.
    unsafe fn extract_cursor_sprite(hcursor: HCURSOR) -> Option<CursorSprite> {
        let mut icon_info = ICONINFO::default();
        let (hot_x, hot_y) = if GetIconInfo(HICON(hcursor.0), &mut icon_info).is_ok() {
            if !icon_info.hbmMask.is_invalid() {
                let _ = DeleteObject(HGDIOBJ(icon_info.hbmMask.0));
            }
            if !icon_info.hbmColor.is_invalid() {
                let _ = DeleteObject(HGDIOBJ(icon_info.hbmColor.0));
            }
            (icon_info.xHotspot as i32, icon_info.yHotspot as i32)
        } else {
            (0, 0)
        };

        let cw = GetSystemMetrics(SM_CXCURSOR).clamp(16, 128);
        let ch = GetSystemMetrics(SM_CYCURSOR).clamp(16, 128);

        let screen_dc = GetDC(None);
        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        let _ = ReleaseDC(None, screen_dc);
        if mem_dc.is_invalid() {
            return None;
        }

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: cw,
                biHeight: -ch, // top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits: *mut c_void = ptr::null_mut();
        let hbm = match CreateDIBSection(Some(mem_dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(bm) if !bm.is_invalid() && !bits.is_null() => bm,
            _ => {
                let _ = DeleteDC(mem_dc);
                return None;
            }
        };

        let prev = SelectObject(mem_dc, HGDIOBJ(hbm.0));
        let byte_len = (cw as usize) * (ch as usize) * 4;
        let dib_slice = slice::from_raw_parts_mut(bits.cast::<u8>(), byte_len);

        // Pass 1: Draw onto pure black (0, 0, 0)
        dib_slice.fill(0);
        let ok1 = DrawIconEx(
            mem_dc,
            0,
            0,
            HICON(hcursor.0),
            cw,
            ch,
            0,
            None,
            DI_NORMAL,
        )
        .is_ok();
        let black = dib_slice.to_vec();

        // Pass 2: Draw onto pure white (255, 255, 255)
        dib_slice.fill(255);
        let ok2 = DrawIconEx(
            mem_dc,
            0,
            0,
            HICON(hcursor.0),
            cw,
            ch,
            0,
            None,
            DI_NORMAL,
        )
        .is_ok();
        let white = dib_slice.to_vec();

        SelectObject(mem_dc, prev);
        let _ = DeleteObject(HGDIOBJ(hbm.0));
        let _ = DeleteDC(mem_dc);

        if !ok1 || !ok2 {
            return None;
        }

        let mut pixels = Vec::with_capacity((cw as usize) * (ch as usize));
        for (bp, wp) in black.chunks_exact(4).zip(white.chunks_exact(4)) {
            let (bb, bg, br) = (bp[0], bp[1], bp[2]);
            let (wb, wg, wr) = (wp[0], wp[1], wp[2]);

            // XOR / inverted cursor pixel (e.g. Windows I-beam text cursor): white becomes dark, black becomes bright.
            if (br > 128 && wr < 128) || (bg > 128 && wg < 128) || (bb > 128 && wb < 128) {
                pixels.push(CursorPixel {
                    r: 0,
                    g: 0,
                    b: 0,
                    alpha: 255,
                    invert: true,
                });
                continue;
            }

            let dr = i16::from(wr) - i16::from(br);
            let dg = i16::from(wg) - i16::from(bg);
            let db = i16::from(wb) - i16::from(bb);
            let inv_alpha = ((dr + dg + db) / 3).clamp(0, 255) as u8;
            let alpha = 255u8.saturating_sub(inv_alpha);
            pixels.push(CursorPixel {
                r: br,
                g: bg,
                b: bb,
                alpha,
                invert: false,
            });
        }

        Some(CursorSprite {
            width: cw as u32,
            height: ch as u32,
            hot_x,
            hot_y,
            pixels,
        })
    }

    struct DxgiCapturer {
        d3d_device: ID3D11Device,
        d3d_context: ID3D11DeviceContext,
        duplication: IDXGIOutputDuplication,
        staging: Option<ID3D11Texture2D>,
        monitor_x: i32,
        monitor_y: i32,
        width: u32,
        height: u32,
        cached_bgra: Vec<u8>,
        has_frame: bool,
    }

    impl DxgiCapturer {
        fn new() -> Option<Self> {
            unsafe {
                let mut d3d_device = None;
                D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None,
                    D3D11_SDK_VERSION,
                    Some(&mut d3d_device),
                    None,
                    None,
                )
                .ok()?;
                let d3d_device = d3d_device?;
                let d3d_context = d3d_device.GetImmediateContext().ok()?;
                let dxgi_device: IDXGIDevice = d3d_device.cast().ok()?;
                let adapter = dxgi_device.GetAdapter().ok()?;

                // Prefer the primary output (DesktopCoordinates origin at (0, 0)), else output 0.
                let mut chosen = None;
                let mut first = None;
                for idx in 0..16 {
                    let Ok(output) = adapter.EnumOutputs(idx) else {
                        break;
                    };
                    let Ok(desc) = output.GetDesc() else { continue };
                    let rect = desc.DesktopCoordinates;
                    let w = (rect.right - rect.left).max(0) as u32;
                    let h = (rect.bottom - rect.top).max(0) as u32;
                    if w == 0 || h == 0 {
                        continue;
                    }
                    if first.is_none() {
                        first = Some((output.clone(), rect.left, rect.top, w, h));
                    }
                    if rect.left == 0 && rect.top == 0 {
                        chosen = Some((output, rect.left, rect.top, w, h));
                        break;
                    }
                }

                let (output, monitor_x, monitor_y, width, height) = chosen.or(first)?;
                let output1: IDXGIOutput1 = output.cast().ok()?;
                let duplication = output1.DuplicateOutput(&dxgi_device).ok()?;

                Some(Self {
                    d3d_device,
                    d3d_context,
                    duplication,
                    staging: None,
                    monitor_x,
                    monitor_y,
                    width,
                    height,
                    cached_bgra: vec![0u8; (width as usize) * (height as usize) * 4],
                    has_frame: false,
                })
            }
        }

        /// Grabs the latest GPU desktop surface (0ms poll once warmed up) into `cached_bgra`.
        /// Returns `Ok(pixels_changed)` or `Err(())` if DXGI needs re-initialization.
        fn grab_latest(&mut self) -> Result<bool, ()> {
            unsafe {
                let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
                let mut resource: Option<IDXGIResource> = None;
                // Wait briefly only for the very first frame; after that poll non-blocking (0ms)
                // so static desktops + cursor movement run at full 60 FPS without waiting!
                let timeout_ms = if self.has_frame { 0 } else { 40 };

                match self
                    .duplication
                    .AcquireNextFrame(timeout_ms, &mut frame_info, &mut resource)
                {
                    Err(err) => {
                        if err.code() == DXGI_ERROR_WAIT_TIMEOUT {
                            return if self.has_frame { Ok(false) } else { Err(()) };
                        }
                        Err(())
                    }
                    Ok(()) => {
                        let mut changed = false;
                        if frame_info.LastPresentTime != 0 {
                            if let Some(res) = resource {
                                if let Ok(src_tex) = res.cast::<ID3D11Texture2D>() {
                                    if self.copy_texture_to_cache(&src_tex).is_ok() {
                                        self.has_frame = true;
                                        changed = true;
                                    }
                                }
                            }
                        }
                        let _ = self.duplication.ReleaseFrame();
                        if !self.has_frame {
                            // Desktop hasn't presented since duplication started — fail over to
                            // seed the initial frame via GDI on the very first tick if needed.
                            return Err(());
                        }
                        Ok(changed)
                    }
                }
            }
        }

        unsafe fn copy_texture_to_cache(&mut self, source_texture: &ID3D11Texture2D) -> Result<(), ()> {
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            source_texture.GetDesc(&mut desc);
            let w = desc.Width;
            let h = desc.Height;
            if w == 0 || h == 0 || w > MAX_SOURCE_WIDTH || h > MAX_SOURCE_HEIGHT {
                return Err(());
            }

            let recreate = match &self.staging {
                None => true,
                Some(tex) => {
                    let mut sdesc = D3D11_TEXTURE2D_DESC::default();
                    tex.GetDesc(&mut sdesc);
                    sdesc.Width != w || sdesc.Height != h || sdesc.Format != desc.Format
                }
            };
            if recreate {
                desc.BindFlags = 0;
                desc.MiscFlags = 0;
                desc.Usage = D3D11_USAGE_STAGING;
                desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                let mut created = None;
                self.d3d_device
                    .CreateTexture2D(&desc, None, Some(&mut created))
                    .map_err(|_| ())?;
                self.staging = created;
                self.width = w;
                self.height = h;
                self.cached_bgra.resize((w as usize) * (h as usize) * 4, 0);
            }

            let staging = self.staging.as_ref().ok_or(())?;
            let dst_res: ID3D11Resource = staging.cast().map_err(|_| ())?;
            let src_res: ID3D11Resource = source_texture.cast().map_err(|_| ())?;
            self.d3d_context.CopyResource(Some(&dst_res), Some(&src_res));

            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.d3d_context
                .Map(Some(&dst_res), 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .map_err(|_| ())?;

            let row_bytes = (w as usize) * 4;
            let pitch = mapped.RowPitch as usize;
            let src_ptr = mapped.pData.cast::<u8>();
            if pitch == row_bytes {
                ptr::copy_nonoverlapping(src_ptr, self.cached_bgra.as_mut_ptr(), row_bytes * h as usize);
            } else {
                for y in 0..h as usize {
                    ptr::copy_nonoverlapping(
                        src_ptr.add(y * pitch),
                        self.cached_bgra.as_mut_ptr().add(y * row_bytes),
                        row_bytes,
                    );
                }
            }

            self.d3d_context.Unmap(Some(&dst_res), 0);
            Ok(())
        }
    }

    /// Reusable top-down DIB GDI fallback (zero per-frame HBITMAP/Vec allocations).
    struct GdiCapturer {
        mem_dc: HDC,
        hbm: HBITMAP,
        prev: HGDIOBJ,
        bits: *mut u8,
        width: u32,
        height: u32,
    }

    // Only accessed on the single screen capture thread.
    unsafe impl Send for GdiCapturer {}

    impl GdiCapturer {
        fn new() -> Option<Self> {
            unsafe {
                let w = GetSystemMetrics(SM_CXSCREEN).max(1) as u32;
                let h = GetSystemMetrics(SM_CYSCREEN).max(1) as u32;
                if w > MAX_SOURCE_WIDTH || h > MAX_SOURCE_HEIGHT {
                    return None;
                }
                let screen_dc = GetDC(None);
                let mem_dc = CreateCompatibleDC(Some(screen_dc));
                let _ = ReleaseDC(None, screen_dc);
                if mem_dc.is_invalid() {
                    return None;
                }
                let bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: w as i32,
                        biHeight: -(h as i32),
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let mut bits: *mut c_void = ptr::null_mut();
                let hbm = match CreateDIBSection(Some(mem_dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
                    Ok(bm) if !bm.is_invalid() && !bits.is_null() => bm,
                    _ => {
                        let _ = DeleteDC(mem_dc);
                        return None;
                    }
                };
                let prev = SelectObject(mem_dc, HGDIOBJ(hbm.0));
                Some(Self {
                    mem_dc,
                    hbm,
                    prev,
                    bits: bits.cast::<u8>(),
                    width: w,
                    height: h,
                })
            }
        }

        fn capture_bgra(&self) -> Option<&[u8]> {
            unsafe {
                let screen_dc = GetDC(None);
                let ok = BitBlt(
                    self.mem_dc,
                    0,
                    0,
                    self.width as i32,
                    self.height as i32,
                    Some(screen_dc),
                    0,
                    0,
                    SRCCOPY,
                )
                .is_ok();
                let _ = ReleaseDC(None, screen_dc);
                if !ok {
                    return None;
                }
                Some(slice::from_raw_parts(
                    self.bits,
                    (self.width as usize) * (self.height as usize) * 4,
                ))
            }
        }
    }

    impl Drop for GdiCapturer {
        fn drop(&mut self) {
            unsafe {
                SelectObject(self.mem_dc, self.prev);
                let _ = DeleteObject(HGDIOBJ(self.hbm.0));
                let _ = DeleteDC(self.mem_dc);
            }
        }
    }

    pub struct PlatformCapturer {
        dxgi: Option<DxgiCapturer>,
        gdi: Option<GdiCapturer>,
        cursor: CursorOverlay,
        last_dxgi_retry: Instant,
    }

    impl PlatformCapturer {
        pub fn new() -> Self {
            Self {
                dxgi: DxgiCapturer::new(),
                gdi: None,
                cursor: CursorOverlay::new(),
                last_dxgi_retry: Instant::now(),
            }
        }

        pub fn reset(&mut self) {
            self.dxgi = None;
            self.gdi = None;
            self.last_dxgi_retry = Instant::now() - Duration::from_secs(5);
        }

        pub fn capture(
            &mut self,
            max_w: u32,
            max_h: u32,
            rgb_out: &mut Vec<u8>,
        ) -> Option<CapturedOutput> {
            // Periodically retry DXGI if we fell back to GDI (e.g. after UAC prompt or mode switch).
            if self.dxgi.is_none() && self.last_dxgi_retry.elapsed() >= Duration::from_millis(1500) {
                self.last_dxgi_retry = Instant::now();
                self.dxgi = DxgiCapturer::new();
            }

            if let Some(dxgi) = self.dxgi.as_mut() {
                match dxgi.grab_latest() {
                    Ok(pixels_changed) => {
                        let (src_w, src_h) = (dxgi.width, dxgi.height);
                        let (dst_w, dst_h) = fit(src_w, src_h, max_w, max_h);
                        if dst_w == 0
                            || dst_h == 0
                            || (dst_w as u64) * (dst_h as u64) > MAX_OUTPUT_PIXELS
                        {
                            return None;
                        }
                        let mut rgba = fast_bgra_to_rgba_and_rgb(
                            &dxgi.cached_bgra,
                            src_w,
                            src_h,
                            dst_w,
                            dst_h,
                            rgb_out,
                        );
                        let cursor_sig = self.cursor.composite(
                            dxgi.monitor_x,
                            dxgi.monitor_y,
                            src_w,
                            src_h,
                            dst_w,
                            dst_h,
                            &mut rgba,
                            rgb_out,
                        );
                        return Some(CapturedOutput {
                            rgba,
                            width: dst_w,
                            height: dst_h,
                            pixels_changed,
                            cursor_sig,
                        });
                    }
                    Err(()) => {
                        // Fall through to GDI for this frame (and re-init DXGI if access was lost).
                        if dxgi.has_frame {
                            self.dxgi = None;
                            self.last_dxgi_retry = Instant::now();
                        }
                    }
                }
            }

            if self.gdi.is_none() {
                self.gdi = GdiCapturer::new();
            }
            let gdi = self.gdi.as_ref()?;
            let bgra = gdi.capture_bgra()?;
            let (src_w, src_h) = (gdi.width, gdi.height);
            let (dst_w, dst_h) = fit(src_w, src_h, max_w, max_h);
            if dst_w == 0 || dst_h == 0 || (dst_w as u64) * (dst_h as u64) > MAX_OUTPUT_PIXELS {
                return None;
            }
            let mut rgba = fast_bgra_to_rgba_and_rgb(bgra, src_w, src_h, dst_w, dst_h, rgb_out);
            let cursor_sig = self.cursor.composite(
                0,
                0,
                src_w,
                src_h,
                dst_w,
                dst_h,
                &mut rgba,
                rgb_out,
            );
            Some(CapturedOutput {
                rgba,
                width: dst_w,
                height: dst_h,
                pixels_changed: true,
                cursor_sig,
            })
        }
    }
}

#[cfg(target_os = "windows")]
use win_capture::PlatformCapturer;

// ============================================================================
// Non-Windows fallback via cached xcap::Monitor
// ============================================================================
#[cfg(not(target_os = "windows"))]
struct PlatformCapturer {
    monitor: Option<xcap::Monitor>,
}

#[cfg(not(target_os = "windows"))]
impl PlatformCapturer {
    fn new() -> Self {
        Self {
            monitor: acquire_primary_monitor(),
        }
    }

    fn reset(&mut self) {
        self.monitor = None;
    }

    fn capture(
        &mut self,
        max_w: u32,
        max_h: u32,
        rgb_out: &mut Vec<u8>,
    ) -> Option<CapturedOutput> {
        if self.monitor.is_none() {
            self.monitor = acquire_primary_monitor();
        }
        let monitor = self.monitor.as_ref()?;
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
        let rgba = fast_rgba_to_rgba_and_rgb(image.as_raw(), src_w, src_h, dst_w, dst_h, rgb_out);
        Some(CapturedOutput {
            rgba,
            width: dst_w,
            height: dst_h,
            pixels_changed: true,
            cursor_sig: (0, 0, 0),
        })
    }
}

#[cfg(not(target_os = "windows"))]
fn acquire_primary_monitor() -> Option<xcap::Monitor> {
    let monitors = xcap::Monitor::all().ok()?;
    let primary = monitors
        .iter()
        .position(|m| m.is_primary().unwrap_or(false))
        .unwrap_or(0);
    monitors.into_iter().nth(primary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_rgba_capture_as_rgb_jpeg() {
        let src = vec![20u8, 40, 200, 255].repeat(32 * 18);
        let (w, h) = fit(32, 18, 1920, 1080);
        let mut rgb = Vec::new();
        let rgba = fast_bgra_to_rgba_and_rgb(&src, 32, 18, w, h, &mut rgb);
        assert_eq!(rgba.len(), (w * h * 4) as usize);
        assert_eq!(rgb.len(), (w * h * 3) as usize);
        assert_eq!(&rgba[..4], &[200, 40, 20, 255]);

        let mut jpeg = Vec::new();
        assert!(encode_rgb_jpeg_420(&rgb, w, h, 60, &mut jpeg));
        assert!(!jpeg.is_empty());
    }

    #[test]
    fn downscales_1440p_to_1080p_and_720p() {
        assert_eq!(fit(2560, 1440, 1920, 1080), (1920, 1080));
        assert_eq!(fit(1920, 1080, 1280, 720), (1280, 720));
        assert_eq!(fit(1920, 1080, 1920, 1080), (1920, 1080));
    }
}
