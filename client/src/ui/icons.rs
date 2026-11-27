//! Uniform Lucide-style vector icons (1.75px stroke, rounded geometry) painted via egui::Painter.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
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

/// <MessageSquare />
#[allow(dead_code)]
pub fn paint_message_square(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let w = size * 0.80;
    let h = size * 0.68;
    let r = egui::Rect::from_center_size(c - Vec2::new(0.0, size * 0.05), Vec2::new(w, h));
    painter.rect_stroke(r, 3.0, s, egui::StrokeKind::Middle);
    painter.line_segment(
        [
            Pos2::new(r.left() + 3.0, r.bottom()),
            Pos2::new(r.left() + 1.0, r.bottom() + size * 0.18),
        ],
        s,
    );
    painter.line_segment(
        [
            Pos2::new(r.left() + 1.0, r.bottom() + size * 0.18),
            Pos2::new(r.left() + size * 0.24, r.bottom()),
        ],
        s,
    );
}

/// <MessageCircle />
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

/// <Phone />
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

/// <PhoneOff />
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

/// <Users />
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

/// <UserPlus />
pub fn paint_user_plus(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
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
    let plus_c = c + Vec2::new(size * 0.26, -size * 0.02);
    let arm = size * 0.13;
    painter.line_segment(
        [plus_c - Vec2::new(arm, 0.0), plus_c + Vec2::new(arm, 0.0)],
        s,
    );
    painter.line_segment(
        [plus_c - Vec2::new(0.0, arm), plus_c + Vec2::new(0.0, arm)],
        s,
    );
}

/// <Compass />
#[allow(dead_code)]
pub fn paint_compass(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    painter.circle_stroke(c, size * 0.38, s);
    let top = c + Vec2::new(size * 0.14, -size * 0.14);
    let bot = c + Vec2::new(-size * 0.14, size * 0.14);
    let l = c + Vec2::new(-size * 0.07, -size * 0.07);
    let r = c + Vec2::new(size * 0.07, size * 0.07);
    painter.line_segment([top, r], s);
    painter.line_segment([r, bot], s);
    painter.line_segment([bot, l], s);
    painter.line_segment([l, top], s);
}

/// <Search />
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

/// <Mic />
pub fn paint_mic(painter: &egui::Painter, c: Pos2, size: f32, color: Color32, muted: bool) {
    let s = stroke(color);
    let capsule = egui::Rect::from_center_size(
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

/// <Headphones />
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
    let left_cup = egui::Rect::from_center_size(
        c + Vec2::new(-size * 0.30, size * 0.16),
        Vec2::new(size * 0.16, size * 0.28),
    );
    let right_cup = egui::Rect::from_center_size(
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

/// <Settings />
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

/// <Inbox />
pub fn paint_inbox(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let r = egui::Rect::from_center_size(c, Vec2::new(size * 0.78, size * 0.64));
    painter.rect_stroke(r, 3.0, s, egui::StrokeKind::Middle);
    let mid_y = r.center().y + size * 0.04;
    let p1 = Pos2::new(r.left(), mid_y);
    let p2 = Pos2::new(r.center().x - size * 0.14, mid_y);
    let p3 = Pos2::new(r.center().x - size * 0.07, mid_y + size * 0.12);
    let p4 = Pos2::new(r.center().x + size * 0.07, mid_y + size * 0.12);
    let p5 = Pos2::new(r.center().x + size * 0.14, mid_y);
    let p6 = Pos2::new(r.right(), mid_y);
    for w in [p1, p2, p3, p4, p5, p6].windows(2) {
        painter.line_segment([w[0], w[1]], s);
    }
}

/// <Download />
pub fn paint_download(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    painter.line_segment(
        [
            c + Vec2::new(0.0, -size * 0.30),
            c + Vec2::new(0.0, size * 0.14),
        ],
        s,
    );
    painter.line_segment(
        [
            c + Vec2::new(-size * 0.18, -size * 0.02),
            c + Vec2::new(0.0, size * 0.16),
        ],
        s,
    );
    painter.line_segment(
        [
            c + Vec2::new(size * 0.18, -size * 0.02),
            c + Vec2::new(0.0, size * 0.16),
        ],
        s,
    );
    painter.line_segment(
        [
            c + Vec2::new(-size * 0.28, size * 0.30),
            c + Vec2::new(size * 0.28, size * 0.30),
        ],
        s,
    );
}

/// <Check />
pub fn paint_check(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let p1 = c + Vec2::new(-size * 0.28, size * 0.02);
    let p2 = c + Vec2::new(-size * 0.06, size * 0.24);
    let p3 = c + Vec2::new(size * 0.30, -size * 0.22);
    painter.line_segment([p1, p2], s);
    painter.line_segment([p2, p3], s);
}

/// <X />
pub fn paint_x(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    let s = stroke(color);
    let d = size * 0.24;
    painter.line_segment([c + Vec2::new(-d, -d), c + Vec2::new(d, d)], s);
    painter.line_segment([c + Vec2::new(d, -d), c + Vec2::new(-d, d)], s);
}

/// <Signal />
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
