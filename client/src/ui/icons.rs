//! Crisp Discord/Lucide-style vector icons + authentic Discord status dots (online, idle crescent, DND bar, offline ring).

use crate::theme;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use skypeline_shared::PresenceStatus;
use std::f32::consts::PI;

fn stroke(color: Color32) -> Stroke {
    Stroke::new(1.75_f32, color)
}

fn paint_arc(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    start_rad: f32,
    end_rad: f32,
    steps: usize,
    s: Stroke,
) {
    let mut prev = center + Vec2::new(start_rad.cos(), start_rad.sin()) * radius;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let a = start_rad + (end_rad - start_rad) * t;
        let next = center + Vec2::new(a.cos(), a.sin()) * radius;
        painter.line_segment([prev, next], s);
        prev = next;
    }
}

/// Authentic Discord 10px status badge with 3px cutout ring in `ring_bg`.
/// For `Online` users (when `!reduced_motion`), draws the 2.5s ease-out green pulse halo.
pub fn paint_status_dot(
    painter: &egui::Painter,
    center: Pos2,
    status: PresenceStatus,
    ring_bg: Color32,
) {
    paint_status_dot_pulse(painter, center, status, ring_bg, None);
}

/// Online status dot with optional 2.5s `onlinePulse` ring (`Some(time_secs)`).
pub fn paint_status_dot_pulse(
    painter: &egui::Painter,
    center: Pos2,
    status: PresenceStatus,
    ring_bg: Color32,
    pulse_time_secs: Option<f64>,
) {
    if status == PresenceStatus::Online {
        if let Some(t) = pulse_time_secs {
            let phase = (t.rem_euclid(2.5) / 2.5) as f32;
            if phase < 0.70 {
                let p = 1.0 - (1.0 - phase / 0.70).powi(3); // ease-out
                let spread = 5.0 * p;
                let alpha = ((1.0 - p) * 0.5 * 255.0).round().clamp(0.0, 255.0) as u8;
                if alpha > 0 {
                    painter.circle_filled(
                        center,
                        5.0 + spread,
                        Color32::from_rgba_unmultiplied(0x23, 0xA5, 0x59, alpha),
                    );
                }
            }
        }
    }
    painter.circle_filled(center, 7.0, ring_bg);
    match status {
        PresenceStatus::Online => {
            painter.circle_filled(center, 5.0, theme::SUCCESS);
        }
        PresenceStatus::Away => {
            painter.circle_filled(center, 5.0, theme::WARNING);
            // Crescent cutout
            painter.circle_filled(center - Vec2::new(2.0, 2.0), 3.3, ring_bg);
        }
        PresenceStatus::Dnd => {
            painter.circle_filled(center, 5.0, theme::DANGER);
            // Horizontal bar cutout
            painter.rect_filled(
                Rect::from_center_size(center, Vec2::new(6.0, 2.0)),
                1.0,
                ring_bg,
            );
        }
        PresenceStatus::Offline => {
            painter.circle_filled(center, 5.0, theme::TEXT_MUTED);
            painter.circle_filled(center, 2.5, ring_bg);
        }
    }
}

pub fn paint_plus(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let r = size * 0.34;
    painter.line_segment([c - Vec2::new(r, 0.0), c + Vec2::new(r, 0.0)], s);
    painter.line_segment([c - Vec2::new(0.0, r), c + Vec2::new(0.0, r)], s);
}

pub fn paint_message_circle(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let r = size * 0.38;
    paint_arc(painter, c, r, -PI * 0.72, PI * 0.85, 20, s);
    let p_end = c + Vec2::new((PI * 0.85).cos(), (PI * 0.85).sin()) * r;
    let p_start = c + Vec2::new((-PI * 0.72).cos(), (-PI * 0.72).sin()) * r;
    let tail = c + Vec2::new(-r * 1.05, r * 0.95);
    painter.line_segment([p_end, tail], s);
    painter.line_segment([tail, p_start], s);
}

pub fn paint_phone(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let pts = [
        c + Vec2::new(-size * 0.28, -size * 0.30),
        c + Vec2::new(-size * 0.12, -size * 0.30),
        c + Vec2::new(-size * 0.07, -size * 0.12),
        c + Vec2::new(-size * 0.15, -size * 0.04),
        c + Vec2::new(size * 0.04, size * 0.15),
        c + Vec2::new(size * 0.12, size * 0.07),
        c + Vec2::new(size * 0.30, size * 0.12),
        c + Vec2::new(size * 0.30, size * 0.28),
        c + Vec2::new(size * 0.16, size * 0.32),
        c + Vec2::new(-size * 0.32, -size * 0.16),
    ];
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], s);
    }
    painter.line_segment([pts[pts.len() - 1], pts[0]], s);
}

pub fn paint_phone_off(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_phone(painter, c, size, color);
    painter.line_segment(
        [
            c + Vec2::new(-size * 0.36, -size * 0.36),
            c + Vec2::new(size * 0.36, size * 0.36),
        ],
        stroke(color),
    );
}

pub fn paint_video(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let body = Rect::from_center_size(
        c - Vec2::new(size * 0.10, 0.0),
        Vec2::new(size * 0.56, size * 0.44),
    );
    painter.rect_stroke(body, 2.5, s, egui::StrokeKind::Middle);
    let p1 = Pos2::new(body.right(), c.y - size * 0.08);
    let p2 = Pos2::new(body.right() + size * 0.22, c.y - size * 0.18);
    let p3 = Pos2::new(body.right() + size * 0.22, c.y + size * 0.18);
    let p4 = Pos2::new(body.right(), c.y + size * 0.08);
    painter.line_segment([p1, p2], s);
    painter.line_segment([p2, p3], s);
    painter.line_segment([p3, p4], s);
}

pub fn paint_pin(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    painter.line_segment(
        [c + Vec2::new(-size * 0.24, -size * 0.24), c + Vec2::new(size * 0.18, -size * 0.24)],
        s,
    );
    painter.line_segment(
        [c + Vec2::new(-size * 0.16, -size * 0.24), c + Vec2::new(-size * 0.24, size * 0.06)],
        s,
    );
    painter.line_segment(
        [c + Vec2::new(size * 0.10, -size * 0.24), c + Vec2::new(size * 0.18, size * 0.06)],
        s,
    );
    painter.line_segment(
        [c + Vec2::new(-size * 0.28, size * 0.06), c + Vec2::new(size * 0.22, size * 0.06)],
        s,
    );
    painter.line_segment(
        [c + Vec2::new(-size * 0.03, size * 0.06), c + Vec2::new(-size * 0.03, size * 0.34)],
        s,
    );
}

pub fn paint_users(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let h1 = c + Vec2::new(-size * 0.12, -size * 0.16);
    painter.circle_stroke(h1, size * 0.16, s);
    paint_arc(
        painter,
        c + Vec2::new(-size * 0.12, size * 0.36),
        size * 0.28,
        PI * 1.12,
        PI * 1.88,
        12,
        s,
    );
    let h2 = c + Vec2::new(size * 0.24, -size * 0.14);
    painter.circle_stroke(h2, size * 0.12, s);
    paint_arc(
        painter,
        c + Vec2::new(size * 0.24, size * 0.34),
        size * 0.22,
        PI * 1.30,
        PI * 1.85,
        8,
        s,
    );
}

pub fn paint_search(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let cc = c - Vec2::new(size * 0.07, size * 0.07);
    painter.circle_stroke(cc, size * 0.27, s);
    painter.line_segment(
        [
            cc + Vec2::new(size * 0.19, size * 0.19),
            c + Vec2::new(size * 0.35, size * 0.35),
        ],
        s,
    );
}

pub fn paint_mic(painter: &egui::Painter, c: Pos2, size: f32, color: Color32, muted: bool) {
    let s = stroke(color);
    let capsule = Rect::from_center_size(
        c - Vec2::new(0.0, size * 0.10),
        Vec2::new(size * 0.26, size * 0.42),
    );
    painter.rect_stroke(capsule, 4.0, s, egui::StrokeKind::Middle);
    paint_arc(
        painter,
        c - Vec2::new(0.0, size * 0.02),
        size * 0.26,
        0.0,
        PI,
        12,
        s,
    );
    painter.line_segment(
        [
            c + Vec2::new(0.0, size * 0.24),
            c + Vec2::new(0.0, size * 0.38),
        ],
        s,
    );
    painter.line_segment(
        [
            c + Vec2::new(-size * 0.15, size * 0.38),
            c + Vec2::new(size * 0.15, size * 0.38),
        ],
        s,
    );
    if muted {
        painter.line_segment(
            [
                c + Vec2::new(-size * 0.34, size * 0.34),
                c + Vec2::new(size * 0.34, -size * 0.34),
            ],
            s,
        );
    }
}

pub fn paint_headphones(
    painter: &egui::Painter,
    c: Pos2,
    size: f32,
    color: Color32,
    deafened: bool,
) {
    let s = stroke(color);
    paint_arc(
        painter,
        c + Vec2::new(0.0, size * 0.06),
        size * 0.33,
        PI,
        PI * 2.0,
        14,
        s,
    );
    let left_cup = Rect::from_center_size(
        c + Vec2::new(-size * 0.30, size * 0.16),
        Vec2::new(size * 0.16, size * 0.28),
    );
    let right_cup = Rect::from_center_size(
        c + Vec2::new(size * 0.30, size * 0.16),
        Vec2::new(size * 0.16, size * 0.28),
    );
    painter.rect_stroke(left_cup, 3.0, s, egui::StrokeKind::Middle);
    painter.rect_stroke(right_cup, 3.0, s, egui::StrokeKind::Middle);
    if deafened {
        painter.line_segment(
            [
                c + Vec2::new(-size * 0.34, size * 0.34),
                c + Vec2::new(size * 0.34, -size * 0.34),
            ],
            s,
        );
    }
}

pub fn paint_settings(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    painter.circle_stroke(c, size * 0.14, s);
    painter.circle_stroke(c, size * 0.29, s);
    for i in 0..6 {
        let a = (i as f32) * (PI / 3.0);
        let p1 = c + Vec2::new(a.cos(), a.sin()) * (size * 0.28);
        let p2 = c + Vec2::new(a.cos(), a.sin()) * (size * 0.40);
        painter.line_segment([p1, p2], s);
    }
}

pub fn paint_smile(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    painter.circle_stroke(c, size * 0.38, s);
    painter.circle_filled(c + Vec2::new(-size * 0.13, -size * 0.08), 1.4, color);
    painter.circle_filled(c + Vec2::new(size * 0.13, -size * 0.08), 1.4, color);
    paint_arc(
        painter,
        c + Vec2::new(0.0, size * 0.02),
        size * 0.18,
        PI * 0.18,
        PI * 0.82,
        10,
        s,
    );
}

pub fn paint_reply(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let tip = c + Vec2::new(-size * 0.30, -size * 0.08);
    painter.line_segment([tip, tip + Vec2::new(size * 0.18, -size * 0.18)], s);
    painter.line_segment([tip, tip + Vec2::new(size * 0.18, size * 0.18)], s);
    painter.line_segment([tip, c + Vec2::new(size * 0.14, -size * 0.08)], s);
    paint_arc(
        painter,
        c + Vec2::new(size * 0.14, size * 0.12),
        size * 0.20,
        -PI * 0.5,
        0.0,
        8,
        s,
    );
}

pub fn paint_pencil(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    painter.line_segment(
        [
            c + Vec2::new(-size * 0.28, size * 0.28),
            c + Vec2::new(size * 0.22, -size * 0.22),
        ],
        s,
    );
    painter.line_segment(
        [
            c + Vec2::new(size * 0.08, -size * 0.32),
            c + Vec2::new(size * 0.32, -size * 0.08),
        ],
        s,
    );
    painter.line_segment(
        [
            c + Vec2::new(-size * 0.32, size * 0.32),
            c + Vec2::new(-size * 0.12, size * 0.30),
        ],
        s,
    );
}

pub fn paint_trash(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    painter.line_segment(
        [
            c + Vec2::new(-size * 0.30, -size * 0.20),
            c + Vec2::new(size * 0.30, -size * 0.20),
        ],
        s,
    );
    painter.line_segment(
        [
            c + Vec2::new(-size * 0.12, -size * 0.32),
            c + Vec2::new(size * 0.12, -size * 0.32),
        ],
        s,
    );
    let body = Rect::from_center_size(
        c + Vec2::new(0.0, size * 0.08),
        Vec2::new(size * 0.44, size * 0.50),
    );
    painter.rect_stroke(body, 2.0, s, egui::StrokeKind::Middle);
}

pub fn paint_check(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let p1 = c + Vec2::new(-size * 0.28, size * 0.02);
    let p2 = c + Vec2::new(-size * 0.06, size * 0.24);
    let p3 = c + Vec2::new(size * 0.30, -size * 0.22);
    painter.line_segment([p1, p2], s);
    painter.line_segment([p2, p3], s);
}

pub fn paint_x(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let d = size * 0.24;
    painter.line_segment([c + Vec2::new(-d, -d), c + Vec2::new(d, d)], s);
    painter.line_segment([c + Vec2::new(d, -d), c + Vec2::new(-d, d)], s);
}

pub fn paint_signal(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = Stroke::new(2.0_f32, color);
    let base_y = c.y + size * 0.30;
    for (idx, h_frac) in [0.22_f32, 0.42, 0.62].iter().enumerate() {
        let x = c.x + (idx as f32 - 1.0) * (size * 0.24);
        painter.line_segment(
            [Pos2::new(x, base_y), Pos2::new(x, base_y - size * h_frac)],
            s,
        );
    }
}

pub fn paint_inbox(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let r = Rect::from_center_size(c, Vec2::new(size * 0.78, size * 0.64));
    painter.rect_stroke(r, 3.0, s, egui::StrokeKind::Middle);
    let mid_y = r.center().y + size * 0.04;
    let pts = [
        Pos2::new(r.left(), mid_y),
        Pos2::new(r.center().x - size * 0.14, mid_y),
        Pos2::new(r.center().x - size * 0.07, mid_y + size * 0.12),
        Pos2::new(r.center().x + size * 0.07, mid_y + size * 0.12),
        Pos2::new(r.center().x + size * 0.14, mid_y),
        Pos2::new(r.right(), mid_y),
    ];
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], s);
    }
}
