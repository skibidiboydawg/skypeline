//! Crisp subpixel-antialiased Phosphor icons (`egui-phosphor`) and authentic Discord status dots.

use crate::theme;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Vec2};
use egui_phosphor::regular;
use skypeline_shared::PresenceStatus;

fn paint_glyph(painter: &egui::Painter, center: Pos2, size: f32, glyph: &str, color: Color32) {
    painter.text(
        center,
        Align2::CENTER_CENTER,
        glyph,
        FontId::new(size, theme::icon_family()),
        color,
    );
}

/// Authentic Discord 10px status badge with 3px cutout ring in `ring_bg`.
pub fn paint_status_dot(
    painter: &egui::Painter,
    center: Pos2,
    status: PresenceStatus,
    ring_bg: Color32,
) {
    paint_status_dot_pulse(painter, center, status, ring_bg, None);
}

/// Online status dot with subtle 2.5s ease-out green halo (`Some(time_secs)`).
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
                let p = 1.0 - (1.0 - phase / 0.70).powi(3);
                let spread = 3.5 * p;
                let alpha = ((1.0 - p) * 0.35 * 255.0).round().clamp(0.0, 255.0) as u8;
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
    painter.circle_filled(center, 6.5, ring_bg);
    match status {
        PresenceStatus::Online => {
            painter.circle_filled(center, 4.5, theme::SUCCESS);
        }
        PresenceStatus::Away => {
            painter.circle_filled(center, 4.5, theme::WARNING);
            painter.circle_filled(center - Vec2::new(1.8, 1.8), 3.0, ring_bg);
        }
        PresenceStatus::Dnd => {
            painter.circle_filled(center, 4.5, theme::DANGER);
            painter.rect_filled(
                Rect::from_center_size(center, Vec2::new(5.5, 1.8)),
                1.0,
                ring_bg,
            );
        }
        PresenceStatus::Offline => {
            painter.circle_filled(center, 4.5, theme::TEXT_MUTED);
            painter.circle_filled(center, 2.2, ring_bg);
        }
    }
}

pub fn paint_plus(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::PLUS, color);
}

pub fn paint_plus_circle(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::PLUS_CIRCLE, color);
}

pub fn paint_message_circle(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::CHAT_CIRCLE, color);
}

pub fn paint_phone(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::PHONE_CALL, color);
}

pub fn paint_phone_off(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::PHONE_DISCONNECT, color);
}

pub fn paint_screen_share(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::MONITOR_ARROW_UP, color);
}

pub fn paint_pin(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::PUSH_PIN, color);
}

pub fn paint_users(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::USERS, color);
}

pub fn paint_search(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::MAGNIFYING_GLASS, color);
}

pub fn paint_mic(painter: &egui::Painter, c: Pos2, size: f32, color: Color32, muted: bool) {
    let glyph = if muted {
        regular::MICROPHONE_SLASH
    } else {
        regular::MICROPHONE
    };
    paint_glyph(painter, c, size, glyph, color);
}

pub fn paint_headphones(
    painter: &egui::Painter,
    c: Pos2,
    size: f32,
    color: Color32,
    deafened: bool,
) {
    let glyph = if deafened {
        regular::SPEAKER_SLASH
    } else {
        regular::HEADPHONES
    };
    paint_glyph(painter, c, size, glyph, color);
}

pub fn paint_settings(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::GEAR_SIX, color);
}

pub fn paint_smile(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::SMILEY, color);
}

pub fn paint_reply(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::ARROW_BEND_UP_LEFT, color);
}

pub fn paint_pencil(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::PENCIL_SIMPLE, color);
}

pub fn paint_trash(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::TRASH, color);
}

pub fn paint_check(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::CHECK, color);
}

pub fn paint_x(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::X, color);
}

pub fn paint_signal(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::CELL_SIGNAL_FULL, color);
}

pub fn paint_inbox(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::TRAY, color);
}

pub fn paint_fullscreen(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::ARROWS_OUT, color);
}

pub fn paint_at(painter: &egui::Painter, c: Pos2, size: f32, color: Color32) {
    paint_glyph(painter, c, size, regular::AT, color);
}
