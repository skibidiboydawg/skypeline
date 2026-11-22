// Native audio / notification bridge for the Skypeline desktop client.
// Kept intentionally dependency-free so it builds with any C++17 compiler.
// No ALSA/PulseAudio symbols are linked: Linux chimes use external players.

#include <atomic>
#include <cmath>
#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <thread>
#include <vector>

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#else
#include <chrono>
#include <cstdint>
#endif

namespace {

std::atomic<float> g_output_gain{1.0f};
std::atomic<unsigned long long> g_tone_counter{0};

float effective_gain() {
    const float gain = g_output_gain.load(std::memory_order_relaxed);
    if (!std::isfinite(gain) || gain < 0.0f) {
        return 0.0f;
    }
    return gain > 2.0f ? 2.0f : gain;
}

// Quote a string for a POSIX shell without allowing injection.
std::string shell_quote(const char* value) {
    std::string out("'");
    for (const char* p = value; *p != '\0'; ++p) {
        if (*p == '\'') {
            out += "'\\''";
        } else {
            out += *p;
        }
    }
    out += "'";
    return out;
}

// Never run system() on a caller thread: a missing player or a slow desktop
// bus must not stall the application.
void run_detached(const std::string& command) {
    if (command.empty()) {
        return;
    }
    std::thread([command]() { std::system(command.c_str()); }).detach();
}

#ifndef _WIN32
// Write a tiny mono 16-bit WAV and hand it to PulseAudio or ALSA. The file is
// removed by the detached worker; failures are ignored on purpose.
void play_tone_linux(int frequency_hz, int duration_ms, int second_hz, int second_ms) {
    if (duration_ms <= 0) {
        return;
    }
    constexpr int kRate = 22050;
    frequency_hz = frequency_hz > 20 && frequency_hz < 20000 ? frequency_hz : 880;
    duration_ms = duration_ms > 0 && duration_ms < 2000 ? duration_ms : 100;
    if (second_hz > 20 && second_hz < 20000 && second_ms > 0 && second_ms < 2000) {
        // keep the requested second tone
    } else {
        second_hz = 0;
        second_ms = 0;
    }

    const int first_samples = kRate * duration_ms / 1000;
    const int second_samples = second_hz > 0 ? kRate * second_ms / 1000 : 0;
    const int total_samples = first_samples + second_samples;
    if (total_samples <= 0) {
        return;
    }

    const auto stamp = static_cast<unsigned long long>(
        std::chrono::steady_clock::now().time_since_epoch().count());
    const std::string path = "/tmp/skypeline_chime_" + std::to_string(stamp) + "_" +
                             std::to_string(g_tone_counter.fetch_add(1)) + ".wav";

    std::FILE* file = std::fopen(path.c_str(), "wb");
    if (file == nullptr) {
        return;
    }

    const uint32_t data_bytes = static_cast<uint32_t>(total_samples * 2);
    const uint32_t riff_bytes = 36u + data_bytes;
    auto write_u16 = [file](uint16_t value) {
        std::fputc(static_cast<int>(value & 0xffu), file);
        std::fputc(static_cast<int>((value >> 8) & 0xffu), file);
    };
    auto write_u32 = [file](uint32_t value) {
        for (int shift = 0; shift < 32; shift += 8) {
            std::fputc(static_cast<int>((value >> shift) & 0xffu), file);
        }
    };

    std::fwrite("RIFF", 1, 4, file);
    write_u32(riff_bytes);
    std::fwrite("WAVEfmt ", 1, 8, file);
    write_u32(16);              // PCM fmt chunk size
    write_u16(1);               // PCM
    write_u16(1);               // mono
    write_u32(kRate);           // sample rate
    write_u32(kRate * 2);       // byte rate
    write_u16(2);               // block align
    write_u16(16);              // bits per sample
    std::fwrite("data", 1, 4, file);
    write_u32(data_bytes);

    const float gain = effective_gain() * 0.18f;
    for (int i = 0; i < total_samples; ++i) {
        const bool second = i >= first_samples;
        const int local = second ? i - first_samples : i;
        const int length = second ? second_samples : first_samples;
        const int freq = second ? second_hz : frequency_hz;
        const double t = static_cast<double>(local) / static_cast<double>(kRate);
        const double tail = (static_cast<double>(length) - static_cast<double>(local)) /
                            static_cast<double>(kRate);
        const double attack = t < 0.01 ? t / 0.01 : 1.0;
        const double release = tail < 0.02 ? tail / 0.02 : 1.0;
        const double envelope = attack < release ? attack : release;
        const double wave = std::sin(2.0 * 3.14159265358979323846 * freq * t);
        const float sample = static_cast<float>(wave * envelope * gain);
        write_u16(static_cast<uint16_t>(static_cast<int16_t>(sample * 32767.0f)));
    }
    std::fclose(file);

    // paplay needs no device config; aplay is the ALSA fallback. Redirect all
    // output so a missing player is completely silent.
    const std::string quoted = "'" + path + "'";
    run_detached("paplay " + quoted + " >/dev/null 2>&1 || aplay -q " + quoted +
                 " >/dev/null 2>&1 || true; rm -f " + quoted);
}

void notify_linux(const char* title, const char* body) {
    const std::string command = "command -v notify-send >/dev/null 2>&1 && notify-send " +
                                shell_quote("--app-name=Skypeline") + " " + shell_quote(title) +
                                " " + shell_quote(body) + " >/dev/null 2>&1 || true";
    run_detached(command);
}
#endif  // !_WIN32

}  // namespace

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

// Playback gain set from the Rust UI. C++ does not render playback audio, so
// this is only used to scale generated chimes; it is safe and atomic.
extern "C" void sp_set_output_volume(float gain) {
    if (!std::isfinite(gain)) {
        return;
    }
    float clamped = gain < 0.0f ? 0.0f : gain;
    if (clamped > 2.0f) {
        clamped = 2.0f;
    }
    g_output_gain.store(clamped, std::memory_order_relaxed);
}

// Simple native alert tone.
extern "C" void sp_beep(int frequency, int duration_ms) {
#ifdef _WIN32
    if (frequency > 0 && duration_ms > 0) {
        ::Beep(static_cast<DWORD>(frequency), static_cast<DWORD>(duration_ms));
    }
#else
    play_tone_linux(frequency, duration_ms, 0, 0);
#endif
}

// Two-tone "new message" cue: bright and short, like a classic IM ping.
extern "C" void sp_message_chime() {
#ifdef _WIN32
    ::Beep(988, 70);
    ::Sleep(45);
    ::Beep(1319, 110);
#else
    play_tone_linux(988, 70, 1319, 110);
#endif
}

// Softer rising cue when someone joins General Voice.
extern "C" void sp_voice_join_chime() {
#ifdef _WIN32
    ::Beep(659, 90);
    ::Sleep(60);
    ::Beep(880, 130);
#else
    play_tone_linux(659, 90, 880, 130);
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
    notify_linux(title, body);
#endif
}
