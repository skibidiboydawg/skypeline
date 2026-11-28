//! Synthesized studio-grade UI sound effects (48 kHz mono PCM) played directly
//! through `AudioEngine` — warm sine/triangle harmonics with smooth ADSR envelopes,
//! zero robotic system beeps, zero external audio assets.

use std::f32::consts::TAU;

const RATE: f32 = 48_000.0;

#[derive(Clone, Copy, Debug)]
pub enum SoundEffect {
    MessageSend,
    MessageReceive,
    CallJoin,
    CallLeave,
    CallIncoming,
    Mute,
    Unmute,
    Deafen,
    Undeafen,
    ScreenShareStart,
    ScreenShareStop,
    Reaction,
    FriendAction,
    UiClick,
}

/// Renders a crisp 48 kHz mono PCM clip for the requested UI interaction.
pub fn synthesize(effect: SoundEffect) -> Vec<f32> {
    match effect {
        SoundEffect::MessageSend => render_glide(420.0, 560.0, 0.055, 0.13),
        SoundEffect::MessageReceive => render_sequence(&[(659.25, 0.075, 0.16), (987.77, 0.115, 0.15)], 0.02),
        SoundEffect::CallJoin => render_sequence(
            &[(523.25, 0.075, 0.17), (659.25, 0.075, 0.17), (783.99, 0.145, 0.18)],
            0.015,
        ),
        SoundEffect::CallLeave => render_sequence(&[(523.25, 0.085, 0.16), (392.00, 0.145, 0.16)], 0.018),
        SoundEffect::CallIncoming => render_sequence(
            &[(783.99, 0.095, 0.18), (987.77, 0.095, 0.18), (1174.66, 0.16, 0.16)],
            0.025,
        ),
        SoundEffect::Mute => render_glide(420.0, 270.0, 0.08, 0.16),
        SoundEffect::Unmute => render_glide(310.0, 480.0, 0.08, 0.16),
        SoundEffect::Deafen => render_sequence(&[(350.0, 0.055, 0.16), (230.0, 0.09, 0.16)], 0.012),
        SoundEffect::Undeafen => render_sequence(&[(290.0, 0.055, 0.16), (460.0, 0.09, 0.16)], 0.012),
        SoundEffect::ScreenShareStart => {
            render_sequence(&[(587.33, 0.07, 0.16), (880.0, 0.13, 0.17)], 0.015)
        }
        SoundEffect::ScreenShareStop => {
            render_sequence(&[(659.25, 0.07, 0.15), (440.0, 0.12, 0.15)], 0.015)
        }
        SoundEffect::Reaction => render_glide(640.0, 920.0, 0.05, 0.12),
        SoundEffect::FriendAction => render_sequence(
            &[(587.33, 0.065, 0.15), (739.99, 0.065, 0.15), (880.0, 0.13, 0.16)],
            0.014,
        ),
        SoundEffect::UiClick => render_glide(540.0, 480.0, 0.026, 0.07),
    }
}

fn render_glide(start_hz: f32, end_hz: f32, secs: f32, gain: f32) -> Vec<f32> {
    let n = (secs * RATE).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(n);
    let mut phase = 0.0f32;
    for i in 0..n {
        let frac = i as f32 / n as f32;
        let hz = start_hz + (end_hz - start_hz) * frac;
        phase += TAU * hz / RATE;
        let attack = (frac / 0.12).min(1.0);
        let decay = (1.0 - frac).powf(1.8);
        let env = attack * decay;
        // Warm fundamental + gentle 2nd harmonic
        let sig = phase.sin() * 0.82 + (phase * 2.0).sin() * 0.18;
        out.push((sig * env * gain).clamp(-0.9, 0.9));
    }
    out
}

fn render_sequence(notes: &[(f32, f32, f32)], gap_secs: f32) -> Vec<f32> {
    let gap = (gap_secs * RATE).round() as usize;
    let total: usize = notes
        .iter()
        .map(|(_, dur, _)| (dur * RATE).round() as usize + gap)
        .sum();
    let mut out = Vec::with_capacity(total);
    for (idx, &(hz, secs, gain)) in notes.iter().enumerate() {
        if idx > 0 {
            out.resize(out.len() + gap, 0.0);
        }
        let n = (secs * RATE).round().max(1.0) as usize;
        for i in 0..n {
            let t = i as f32 / RATE;
            let frac = i as f32 / n as f32;
            let attack = (frac / 0.08).min(1.0);
            let decay = (-frac * 3.8).exp() * (1.0 - frac).powf(0.4);
            let env = attack * decay;
            let sig = (TAU * hz * t).sin() * 0.80
                + (TAU * hz * 2.0 * t).sin() * 0.14
                + (TAU * hz * 3.0 * t).sin() * 0.06;
            out.push((sig * env * gain).clamp(-0.9, 0.9));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesizes_finite_nonempty_clips() {
        for fx in [
            SoundEffect::MessageSend,
            SoundEffect::MessageReceive,
            SoundEffect::CallJoin,
            SoundEffect::CallLeave,
            SoundEffect::Mute,
            SoundEffect::ScreenShareStart,
            SoundEffect::Reaction,
            SoundEffect::UiClick,
        ] {
            let pcm = synthesize(fx);
            assert!(!pcm.is_empty());
            assert!(pcm.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
        }
    }
}
