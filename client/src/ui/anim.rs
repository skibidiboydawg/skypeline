//! Smooth 60 FPS CSS-equivalent animation curves and keyframe evaluators for egui.
//! Every helper respects `reduced_motion` by snapping instantaneously to the final state.

/// Standard CSS `ease` / `cubic-bezier(0.4, 0, 0.2, 1)` approximation.
pub fn ease_standard(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    // Smooth cubic-bezier(0.4, 0.0, 0.2, 1.0)
    cubic_bezier_y(t, 0.4, 0.0, 0.2, 1.0)
}

/// Standard CSS `ease-out`.
pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Standard CSS `ease-in`.
pub fn ease_in(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t
}

/// Exact cubic-bezier evaluator (solves x(u) = t via Newton-Raphson, returns y(u)).
pub fn cubic_bezier_y(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let mut u = x;
    for _ in 0..6 {
        let bx = bezier_sample(u, x1, x2) - x;
        let dx = bezier_deriv(u, x1, x2);
        if dx.abs() < 1e-5 {
            break;
        }
        u = (u - bx / dx).clamp(0.0, 1.0);
    }
    bezier_sample(u, y1, y2)
}

fn bezier_sample(u: f32, p1: f32, p2: f32) -> f32 {
    let inv = 1.0 - u;
    3.0 * inv * inv * u * p1 + 3.0 * inv * u * u * p2 + u * u * u
}

fn bezier_deriv(u: f32, p1: f32, p2: f32) -> f32 {
    let inv = 1.0 - u;
    3.0 * inv * inv * p1 + 6.0 * inv * u * (p2 - p1) + 3.0 * u * u * (1.0 - p2)
}

/// #2 Message send/receive spring bounce (`messageIn`, 200ms, `cubic-bezier(0.34, 1.56, 0.64, 1)`).
/// Returns `(opacity, y_offset_px, scale, done)`.
pub fn message_in(elapsed: f32, reduced_motion: bool) -> (f32, f32, f32, bool) {
    if reduced_motion || elapsed >= 0.20 {
        return (1.0, 0.0, 1.0, true);
    }
    let raw = (elapsed / 0.20).clamp(0.0, 1.0);
    // Piecewise keyframes with spring feel:
    // 0% -> 60%: opacity 0..1, y +8..-2, scale 0.97..1.01
    // 60% -> 100%: opacity 1, y -2..0, scale 1.01..1.0
    if raw <= 0.60 {
        let p = ease_out(raw / 0.60);
        let opacity = p;
        let y = 8.0 + (-2.0 - 8.0) * p;
        let scale = 0.97 + (1.01 - 0.97) * p;
        (opacity, y, scale, false)
    } else {
        let p = ease_standard((raw - 0.60) / 0.40);
        let y = -2.0 + 2.0 * p;
        let scale = 1.01 + (1.0 - 1.01) * p;
        (1.0, y, scale, false)
    }
}

/// #4 Typing indicator dot vertical offset (`typingBounce`, 1.2s loop, staggered by `delay_secs`).
/// Peak `-5px` at 30%, resting at `0px` from 60%..=100%.
pub fn typing_dot_offset(time_secs: f64, delay_secs: f64, reduced_motion: bool) -> f32 {
    if reduced_motion {
        return 0.0;
    }
    let period = 1.2_f64;
    let phase = ((time_secs - delay_secs).rem_euclid(period) / period) as f32;
    if phase < 0.30 {
        let p = ease_standard(phase / 0.30);
        -5.0 * p
    } else if phase < 0.60 {
        let p = ease_standard((phase - 0.30) / 0.30);
        -5.0 * (1.0 - p)
    } else {
        0.0
    }
}

/// #6 Pending badge pop (`badgePop`, 300ms, `cubic-bezier(0.34, 1.56, 0.64, 1)`).
/// `0%`: 0.5 -> `70%`: 1.15 -> `100%`: 1.0.
pub fn badge_pop_scale(elapsed: f32, reduced_motion: bool) -> (f32, bool) {
    if reduced_motion || elapsed >= 0.30 {
        return (1.0, true);
    }
    let raw = (elapsed / 0.30).clamp(0.0, 1.0);
    if raw <= 0.70 {
        let p = ease_out(raw / 0.70);
        (0.5 + (1.15 - 0.5) * p, false)
    } else {
        let p = ease_standard((raw - 0.70) / 0.30);
        (1.15 + (1.0 - 1.15) * p, false)
    }
}

/// #8 Modal/popup enter (180ms) and exit (150ms).
/// Returns `(opacity, scale, backdrop_opacity, finished_exit)`.
pub fn modal_transition(
    open_elapsed: f32,
    close_elapsed: Option<f32>,
    reduced_motion: bool,
) -> (f32, f32, f32, bool) {
    if let Some(ce) = close_elapsed {
        if reduced_motion || ce >= 0.15 {
            return (0.0, 0.95, 0.0, true);
        }
        let p = ease_standard((ce / 0.15).clamp(0.0, 1.0));
        let alpha = 1.0 - p;
        let scale = 1.0 - 0.05 * p;
        return (alpha, scale, alpha, false);
    }
    if reduced_motion || open_elapsed >= 0.18 {
        return (1.0, 1.0, 1.0, false);
    }
    let raw = (open_elapsed / 0.18).clamp(0.0, 1.0);
    let p = ease_standard(raw);
    (p, 0.95 + 0.05 * p, raw, false)
}

/// #9 Toast enter (250ms) and exit (200ms).
/// Returns `(opacity, y_offset_px, scale, finished_exit)`.
pub fn toast_transition(
    open_elapsed: f32,
    close_elapsed: Option<f32>,
    reduced_motion: bool,
) -> (f32, f32, f32, bool) {
    if let Some(ce) = close_elapsed {
        if reduced_motion || ce >= 0.20 {
            return (0.0, -8.0, 1.0, true);
        }
        let p = ease_standard((ce / 0.20).clamp(0.0, 1.0));
        return (1.0 - p, -8.0 * p, 1.0, false);
    }
    if reduced_motion || open_elapsed >= 0.25 {
        return (1.0, 0.0, 1.0, false);
    }
    let p = ease_standard((open_elapsed / 0.25).clamp(0.0, 1.0));
    (p, -16.0 * (1.0 - p), 0.97 + 0.03 * p, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduced_motion_snaps_instantly() {
        assert_eq!(message_in(0.01, true), (1.0, 0.0, 1.0, true));
        assert_eq!(badge_pop_scale(0.01, true), (1.0, true));
        assert_eq!(typing_dot_offset(0.36, 0.0, true), 0.0);
        assert_eq!(modal_transition(0.01, None, true), (1.0, 1.0, 1.0, false));
        assert_eq!(modal_transition(1.0, Some(0.01), true), (0.0, 0.95, 0.0, true));
        assert_eq!(toast_transition(0.01, None, true), (1.0, 0.0, 1.0, false));
        assert_eq!(toast_transition(1.0, Some(0.01), true), (0.0, -8.0, 1.0, true));
    }

    #[test]
    fn message_in_overshoots_then_settles() {
        let (op0, y0, _, done0) = message_in(0.0, false);
        assert!(!done0 && op0 < 0.01 && (y0 - 8.0).abs() < 0.1);
        let (op_mid, y_mid, sc_mid, done_mid) = message_in(0.12, false);
        assert!(!done_mid && op_mid > 0.9 && y_mid < 0.0 && sc_mid > 1.0);
        let (op1, y1, sc1, done1) = message_in(0.21, false);
        assert!(done1 && (op1 - 1.0).abs() < 1e-4 && y1.abs() < 1e-4 && (sc1 - 1.0).abs() < 1e-4);
    }
}
