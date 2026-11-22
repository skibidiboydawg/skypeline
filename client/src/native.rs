//! Thin safe wrappers around the C++ audio / notification bridge in `cpp/`.

use std::ffi::CString;

unsafe extern "C" {
    fn sp_audio_peak(samples: *const f32, count: i32) -> f32;
    fn sp_message_chime();
    fn sp_voice_join_chime();
    fn sp_set_output_volume(gain: f32);
    fn sp_notify(title: *const std::ffi::c_char, body: *const std::ffi::c_char);
}

/// Peak amplitude of a mono PCM frame (used for voice activity detection).
pub fn audio_peak(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    unsafe { sp_audio_peak(samples.as_ptr(), samples.len() as i32) }
}

/// Crisp two-tone cue for an incoming chat message.
pub fn message_chime() {
    std::thread::spawn(|| unsafe { sp_message_chime() });
}

/// Subtle rising cue when someone joins the voice room.
pub fn voice_join_chime() {
    std::thread::spawn(|| unsafe { sp_voice_join_chime() });
}

/// Set the playback gain used by the voice stage volume slider.
pub fn set_output_volume(gain: f32) {
    unsafe { sp_set_output_volume(gain.clamp(0.0, 2.0)) }
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
