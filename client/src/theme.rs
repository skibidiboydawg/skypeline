//! Polished dark-navy palette and typography for egui.

use eframe::egui::{Color32, Context, FontId, RichText, Stroke, TextStyle, Visuals};

pub const PRIMARY_TEXT: Color32 = Color32::from_rgb(0xF3, 0xF4, 0xF6);
pub const SECONDARY_TEXT: Color32 = Color32::from_rgb(0x9C, 0xA3, 0xAF);
pub const SURFACE_SIDEBAR: Color32 = Color32::from_rgb(0x0B, 0x0F, 0x19);
pub const SURFACE_MAIN: Color32 = Color32::from_rgb(0x11, 0x18, 0x27);
pub const SURFACE_CARD: Color32 = Color32::from_rgb(0x1F, 0x29, 0x37);
pub const BORDER: Color32 = Color32::from_rgb(0x1F, 0x29, 0x37);
pub const ACCENT: Color32 = Color32::from_rgb(0x3B, 0x82, 0xF6);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0x1D, 0x3A, 0x63);
pub const HOVER: Color32 = Color32::from_rgb(0x27, 0x35, 0x47);
pub const DANGER: Color32 = Color32::from_rgb(0xEF, 0x44, 0x44);
pub const SUCCESS: Color32 = Color32::from_rgb(0x22, 0xC5, 0x5E);

pub fn border() -> Stroke {
    Stroke::new(1.0_f32, BORDER)
}

/// Secondary/helper text with the exact muted color from the design spec.
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
            Color32::from_rgb(0xF8, 0xFA, 0xFC),
            Color32::from_rgb(0xFF, 0xFF, 0xFF),
            Color32::from_rgb(0xF1, 0xF5, 0xF9),
            Color32::from_rgb(0xE2, 0xE8, 0xF0),
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
        widget.corner_radius = 8.0.into();
    }
    visuals.widgets.hovered.bg_fill = hover;
    visuals.widgets.active.bg_fill = ACCENT;

    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(13.0));
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(15.0));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(21.0));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::proportional(14.0));
        style.visuals.window_corner_radius = 12.0.into();
    });
}
