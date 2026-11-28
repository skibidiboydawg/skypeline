//! Thin safe wrappers around the C++ audio bridge in `cpp/`.

unsafe extern "C" {
    fn sp_audio_peak(samples: *const f32, count: i32) -> f32;
}

/// Peak amplitude of a mono PCM frame (used for voice activity detection).
pub fn audio_peak(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    unsafe { sp_audio_peak(samples.as_ptr(), samples.len() as i32) }
}

/// Native Windows MessageBox notifications disabled — Wireline uses non-intrusive
/// in-app toasts and audio alerts instead.
#[allow(dead_code)]
pub fn notify(_title: &str, _body: &str) {}
