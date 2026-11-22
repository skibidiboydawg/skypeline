//! Thin safe wrappers around the C++ audio / notification bridge in `cpp/`.

use std::ffi::CString;

unsafe extern "C" {
    fn sp_audio_peak(samples: *const f32, count: i32) -> f32;
    fn sp_beep(frequency: i32, duration_ms: i32);
    fn sp_notify(title: *const std::ffi::c_char, body: *const std::ffi::c_char);
}

/// Peak amplitude of a mono PCM frame (used for voice activity detection).
pub fn audio_peak(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    unsafe { sp_audio_peak(samples.as_ptr(), samples.len() as i32) }
}

/// Short native alert tone.
pub fn beep() {
    // 880 Hz for 90 ms — a soft "you've got a message" chime.
    unsafe { sp_beep(880, 90) }
}

/// Show a native desktop alert. Runs on its own thread because the C++
/// implementation may use a modal message box on Windows.
pub fn notify(title: &str, body: &str) {
    let title = title.to_string();
    let body = body.to_string();
    std::thread::spawn(move || {
        let Ok(t) = CString::new(title) else { return };
        let Ok(b) = CString::new(body) else { return };
        unsafe { sp_notify(t.as_ptr(), b.as_ptr()) }
    });
}
