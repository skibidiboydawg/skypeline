//! Wireline obsidian-slate + electric-indigo design system.

use eframe::egui::{Color32, Context, FontId, RichText, Stroke, TextStyle, Visuals};

pub const PRIMARY_TEXT: Color32 = Color32::from_rgb(0xFA, 0xFA, 0xFA);
pub const SECONDARY_TEXT: Color32 = Color32::from_rgb(0xA1, 0xA1, 0xAA);
pub const MUTED_TEXT: Color32 = Color32::from_rgb(0x71, 0x71, 0x7A);

pub const SURFACE_RAIL: Color32 = Color32::from_rgb(0x09, 0x09, 0x0B);
pub const SURFACE_SIDEBAR: Color32 = Color32::from_rgb(0x11, 0x11, 0x15);
pub const SURFACE_MAIN: Color32 = Color32::from_rgb(0x14, 0x14, 0x19);
pub const SURFACE_CARD: Color32 = Color32::from_rgb(0x1C, 0x1C, 0x22);
pub const SURFACE_ELEVATED: Color32 = Color32::from_rgb(0x23, 0x23, 0x2B);

pub const BORDER: Color32 = Color32::from_rgb(0x27, 0x27, 0x2F);
pub const INPUT_BORDER: Color32 = Color32::from_rgb(0x3F, 0x3F, 0x46);

pub const ACCENT: Color32 = Color32::from_rgb(0x63, 0x66, 0xF1);
pub const ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(99, 102, 241, 38);
pub const CYAN_GLOW: Color32 = Color32::from_rgb(0x22, 0xD3, 0xEE);

pub const HOVER: Color32 = Color32::from_rgb(0x24, 0x24, 0x2E);
pub const DANGER: Color32 = Color32::from_rgb(0xEF, 0x44, 0x44);
pub const SUCCESS: Color32 = Color32::from_rgb(0x10, 0xB9, 0x81);
pub const WARNING: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);

pub fn border() -> Stroke {
    Stroke::new(1.0_f32, BORDER)
}

pub fn input_border() -> Stroke {
    Stroke::new(1.0_f32, INPUT_BORDER)
}

pub fn accent_border() -> Stroke {
    Stroke::new(1.0_f32, ACCENT)
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
            Color32::from_rgb(0x18, 0x18, 0x1B),
            Color32::from_rgb(0xF4, 0xF4, 0xF5),
            Color32::from_rgb(0xFF, 0xFF, 0xFF),
            Color32::from_rgb(0xE4, 0xE4, 0xE7),
            Color32::from_rgb(0xD4, 0xD4, 0xD8),
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
    visuals.hyperlink_color = CYAN_GLOW;
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
            .insert(TextStyle::Body, FontId::proportional(14.5));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(14.0));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(21.0));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::proportional(13.5));
        style.spacing.item_spacing = eframe::egui::vec2(8.0, 6.0);
        style.visuals.window_corner_radius = 14.0.into();
    });
}
