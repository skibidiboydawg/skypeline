// Native audio / notification bridge for the Skypeline desktop client.
// Kept intentionally dependency-free so it builds with any C++17 compiler.

#include <cmath>
#include <cstddef>

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#endif

// Peak amplitude of a PCM frame. Used by the Rust side for voice activity
// detection without allocating in the cpal real-time callback.
extern "C" float sp_audio_peak(const float* samples, int count) {
    if (samples == nullptr || count <= 0) {
        return 0.0f;
    }
    float peak = 0.0f;
    for (int i = 0; i < count; ++i) {
        const float v = samples[i];
        if (!std::isfinite(v)) {
            continue;
        }
        const float a = v < 0.0f ? -v : v;
        if (a > peak) {
            peak = a;
        }
    }
    return peak;
}

// Simple native alert tone (Windows Beep; no-op elsewhere).
extern "C" void sp_beep(int frequency, int duration_ms) {
#ifdef _WIN32
    if (frequency > 0 && duration_ms > 0) {
        ::Beep(static_cast<DWORD>(frequency), static_cast<DWORD>(duration_ms));
    }
#else
    (void)frequency;
    (void)duration_ms;
#endif
}

// Native desktop notification. Called from a background Rust thread so the
// egui event loop is never blocked by a modal dialog.
extern "C" void sp_notify(const char* title, const char* body) {
    if (title == nullptr || body == nullptr) {
        return;
    }
#ifdef _WIN32
    ::MessageBoxA(nullptr, body, title, MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND);
#else
    (void)title;
    (void)body;
#endif
}
