use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use std::time::Instant;

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

fn main() {
    for i in 0..5 {
        let start = Instant::now();
        let monitors = xcap::Monitor::all().expect("enumerate monitors");
        eprintln!("monitors={} elapsed={:?}", monitors.len(), start.elapsed());
        let monitor = monitors
            .iter()
            .find(|m| m.is_primary().unwrap_or(false))
            .or_else(|| monitors.first())
            .expect("no monitor");
        eprintln!(
            "monitor size={}x{} primary={}",
            monitor.width().unwrap_or(0),
            monitor.height().unwrap_or(0),
            monitor.is_primary().unwrap_or(false)
        );
        let img = monitor.capture_image().expect("capture image");
        eprintln!(
            "captured={}x{} elapsed={:?}",
            img.width(),
            img.height(),
            start.elapsed()
        );
        let (w, h) = fit(img.width(), img.height(), 1280, 720);
        let resized = image::imageops::resize(&img, w, h, FilterType::Triangle);
        let rgb = image::DynamicImage::ImageRgba8(resized).to_rgb8();
        let mut jpeg = Vec::with_capacity((w * h / 4) as usize);
        JpegEncoder::new_with_quality(&mut jpeg, 62)
            .encode(rgb.as_raw(), w, h, image::ExtendedColorType::Rgb8)
            .expect("encode jpeg");
        eprintln!(
            "frame={i} jpeg={} bytes elapsed={:?}",
            jpeg.len(),
            start.elapsed()
        );
        std::thread::sleep(std::time::Duration::from_millis(33));
    }
}
