//! Wireline high-craft dark-mode palette (#0f1117, #161922, #1e222d, #6366f1).

use eframe::egui::{Color32, Context, FontId, RichText, Stroke, TextStyle, Visuals};

pub const PRIMARY_TEXT: Color32 = Color32::from_rgb(0xF3, 0xF4, 0xF6);
pub const SECONDARY_TEXT: Color32 = Color32::from_rgb(0x9C, 0xA3, 0xAF);
pub const MUTED_TEXT: Color32 = Color32::from_rgb(0x6B, 0x72, 0x80);

pub const SURFACE_RAIL: Color32 = Color32::from_rgb(0x0F, 0x11, 0x17);
pub const SURFACE_SIDEBAR: Color32 = Color32::from_rgb(0x16, 0x19, 0x22);
pub const SURFACE_MAIN: Color32 = Color32::from_rgb(0x0F, 0x11, 0x17);
pub const SURFACE_CARD: Color32 = Color32::from_rgb(0x1E, 0x22, 0x2D);
pub const SURFACE_ELEVATED: Color32 = Color32::from_rgb(0x26, 0x2B, 0x38);
pub const SURFACE_DOCK: Color32 = Color32::from_rgb(0x12, 0x15, 0x1D);

pub const BORDER: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 20);
pub const INPUT_BORDER: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 32);

pub const ACCENT: Color32 = Color32::from_rgb(0x63, 0x66, 0xF1);
pub const ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(99, 102, 241, 36);
pub const CYAN_GLOW: Color32 = Color32::from_rgb(0x22, 0xD3, 0xEE);

pub const HOVER: Color32 = Color32::from_rgb(0x23, 0x28, 0x35);
pub const DANGER: Color32 = Color32::from_rgb(0xEF, 0x44, 0x44);
pub const SUCCESS: Color32 = Color32::from_rgb(0x22, 0xC5, 0x5E);
pub const WARNING: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);

pub fn border() -> Stroke {
    Stroke::new(1.0_f32, BORDER)
}

pub fn input_border() -> Stroke {
    Stroke::new(1.0_f32, INPUT_BORDER)
}

pub fn accent_border() -> Stroke {
    Stroke::new(1.5_f32, ACCENT)
}

pub fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text).color(SECONDARY_TEXT)
}

pub fn apply(ctx: &Context, dark_mode: bool) {
    let mut visuals = if dark_mode {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    let (primary, panel, window, card, hover) = if dark_mode {
        (
            PRIMARY_TEXT,
            SURFACE_SIDEBAR,
            SURFACE_MAIN,
            SURFACE_CARD,
            HOVER,
        )
    } else {
        (
            Color32::from_rgb(0x11, 0x18, 0x27),
            Color32::from_rgb(0xF3, 0xF4, 0xF6),
            Color32::from_rgb(0xFF, 0xFF, 0xFF),
            Color32::from_rgb(0xE5, 0xE7, 0xEB),
            Color32::from_rgb(0xD1, 0xD5, 0xDB),
        )
    };

    visuals.override_text_color = Some(primary);
    visuals.panel_fill = panel;
    visuals.window_fill = window;
    visuals.window_stroke = border();
    visuals.extreme_bg_color = card;
    visuals.faint_bg_color = card;
    visuals.code_bg_color = card;
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke.color = ACCENT;
    visuals.hyperlink_color = ACCENT;
    visuals.widgets.hovered.bg_fill = hover;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.open.bg_fill = card;

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.bg_fill = card;
        widget.weak_bg_fill = card;
        widget.bg_stroke = border();
        widget.fg_stroke.color = primary;
        widget.corner_radius = 10.0.into();
    }
    visuals.widgets.hovered.bg_fill = hover;
    visuals.widgets.active.bg_fill = ACCENT;

    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(12.5));
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(14.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(13.5));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(20.0));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::proportional(13.0));
        style.spacing.item_spacing = eframe::egui::vec2(8.0, 6.0);
        style.visuals.window_corner_radius = 12.0.into();
    });
}
