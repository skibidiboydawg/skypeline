//! Crisp Lucide-style vector icons painted via egui::Painter (no missing font glyphs).

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

pub fn paint_chat_icon(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let r = size * 0.42;
    let rect = egui::Rect::from_center_size(center - egui::vec2(0.0, 1.0), egui::vec2(r * 2.0, r * 1.65));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.6_f32, color), egui::StrokeKind::Middle);
    let tail = [
        Pos2::new(rect.left() + 4.0, rect.bottom() - 1.0),
        Pos2::new(rect.left() + 2.0, rect.bottom() + 4.0),
        Pos2::new(rect.left() + 8.0, rect.bottom() - 1.0),
    ];
    painter.line_segment([tail[0], tail[1]], Stroke::new(1.5_f32, color));
    painter.line_segment([tail[1], tail[2]], Stroke::new(1.5_f32, color));
}

pub fn paint_phone_icon(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let s = Stroke::new(1.7_f32, color);
    let pts = [
        center + Vec2::new(-size * 0.28, -size * 0.30),
        center + Vec2::new(-size * 0.14, -size * 0.30),
        center + Vec2::new(-size * 0.08, -size * 0.12),
        center + Vec2::new(-size * 0.16, -size * 0.04),
        center + Vec2::new(size * 0.04, size * 0.16),
        center + Vec2::new(size * 0.12, size * 0.08),
        center + Vec2::new(size * 0.30, size * 0.14),
        center + Vec2::new(size * 0.30, size * 0.28),
        center + Vec2::new(size * 0.14, size * 0.32),
        center + Vec2::new(-size * 0.32, -size * 0.14),
    ];
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], s);
    }
    painter.line_segment([pts[pts.len() - 1], pts[0]], s);
}

pub fn paint_users_icon(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let s = Stroke::new(1.6_f32, color);
    let head = center + Vec2::new(-size * 0.10, -size * 0.16);
    painter.circle_stroke(head, size * 0.17, s);
    let body = egui::Rect::from_center_size(
        center + Vec2::new(-size * 0.10, size * 0.20),
        Vec2::new(size * 0.50, size * 0.26),
    );
    painter.rect_stroke(body, 4.0, s, egui::StrokeKind::Middle);
    let head2 = center + Vec2::new(size * 0.24, -size * 0.14);
    painter.circle_stroke(head2, size * 0.12, s);
}

pub fn paint_sun_moon_icon(
    painter: &egui::Painter,
    center: Pos2,
    size: f32,
    color: Color32,
    dark_mode: bool,
) {
    let s = Stroke::new(1.6_f32, color);
    if dark_mode {
        painter.circle_stroke(center, size * 0.22, s);
        for i in 0..8 {
            let a = (i as f32) * std::f32::consts::FRAC_PI_4;
            let p1 = center + Vec2::new(a.cos(), a.sin()) * (size * 0.31);
            let p2 = center + Vec2::new(a.cos(), a.sin()) * (size * 0.41);
            painter.line_segment([p1, p2], s);
        }
    } else {
        painter.circle_stroke(center, size * 0.30, s);
    }
}

pub fn paint_search_icon(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let s = Stroke::new(1.5_f32, color);
    let c = center - Vec2::new(size * 0.08, size * 0.08);
    painter.circle_stroke(c, size * 0.26, s);
    painter.line_segment(
        [
            c + Vec2::new(size * 0.18, size * 0.18),
            center + Vec2::new(size * 0.34, size * 0.34),
        ],
        s,
    );
}

pub fn paint_mic_icon(
    painter: &egui::Painter,
    center: Pos2,
    size: f32,
    color: Color32,
    muted: bool,
) {
    let s = Stroke::new(1.5_f32, color);
    let capsule = egui::Rect::from_center_size(
        center - Vec2::new(0.0, size * 0.08),
        Vec2::new(size * 0.28, size * 0.44),
    );
    painter.rect_stroke(capsule, 4.0, s, egui::StrokeKind::Middle);
    painter.line_segment(
        [
            center + Vec2::new(0.0, size * 0.22),
            center + Vec2::new(0.0, size * 0.36),
        ],
        s,
    );
    painter.line_segment(
        [
            center + Vec2::new(-size * 0.16, size * 0.36),
            center + Vec2::new(size * 0.16, size * 0.36),
        ],
        s,
    );
    if muted {
        painter.line_segment(
            [
                center + Vec2::new(-size * 0.34, size * 0.34),
                center + Vec2::new(size * 0.34, -size * 0.34),
            ],
            Stroke::new(1.7_f32, color),
        );
    }
}

pub fn paint_headphones_icon(
    painter: &egui::Painter,
    center: Pos2,
    size: f32,
    color: Color32,
    deafened: bool,
) {
    let s = Stroke::new(1.5_f32, color);
    let left = center + Vec2::new(-size * 0.28, size * 0.10);
    let right = center + Vec2::new(size * 0.28, size * 0.10);
    let top = center + Vec2::new(0.0, -size * 0.28);
    painter.line_segment([left, center + Vec2::new(-size * 0.26, -size * 0.14)], s);
    painter.line_segment([center + Vec2::new(-size * 0.26, -size * 0.14), top], s);
    painter.line_segment([top, center + Vec2::new(size * 0.26, -size * 0.14)], s);
    painter.line_segment([center + Vec2::new(size * 0.26, -size * 0.14), right], s);
    painter.rect_filled(
        egui::Rect::from_center_size(left, Vec2::new(size * 0.16, size * 0.28)),
        2.5,
        color,
    );
    painter.rect_filled(
        egui::Rect::from_center_size(right, Vec2::new(size * 0.16, size * 0.28)),
        2.5,
        color,
    );
    if deafened {
        painter.line_segment(
            [
                center + Vec2::new(-size * 0.34, size * 0.34),
                center + Vec2::new(size * 0.34, -size * 0.34),
            ],
            Stroke::new(1.7_f32, color),
        );
    }
}

pub fn paint_gear_icon(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let s = Stroke::new(1.5_f32, color);
    painter.circle_stroke(center, size * 0.16, s);
    painter.circle_stroke(center, size * 0.30, s);
    for i in 0..6 {
        let a = (i as f32) * std::f32::consts::FRAC_PI_3;
        let p1 = center + Vec2::new(a.cos(), a.sin()) * (size * 0.29);
        let p2 = center + Vec2::new(a.cos(), a.sin()) * (size * 0.40);
        painter.line_segment([p1, p2], s);
    }
}

pub fn paint_bell_icon(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let s = Stroke::new(1.5_f32, color);
    let body = egui::Rect::from_center_size(
        center - Vec2::new(0.0, size * 0.04),
        Vec2::new(size * 0.48, size * 0.50),
    );
    painter.rect_stroke(body, 5.0, s, egui::StrokeKind::Middle);
    painter.line_segment(
        [
            center + Vec2::new(-size * 0.34, size * 0.22),
            center + Vec2::new(size * 0.34, size * 0.22),
        ],
        s,
    );
    painter.circle_filled(center + Vec2::new(0.0, size * 0.32), 1.8, color);
}

pub fn paint_download_icon(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let s = Stroke::new(1.6_f32, color);
    painter.line_segment(
        [
            center + Vec2::new(0.0, -size * 0.30),
            center + Vec2::new(0.0, size * 0.14),
        ],
        s,
    );
    painter.line_segment(
        [
            center + Vec2::new(-size * 0.18, -size * 0.02),
            center + Vec2::new(0.0, size * 0.16),
        ],
        s,
    );
    painter.line_segment(
        [
            center + Vec2::new(size * 0.18, -size * 0.02),
            center + Vec2::new(0.0, size * 0.16),
        ],
        s,
    );
    painter.line_segment(
        [
            center + Vec2::new(-size * 0.28, size * 0.30),
            center + Vec2::new(size * 0.28, size * 0.30),
        ],
        s,
    );
}
