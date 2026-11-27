//! Pixel-faithful Discord dark UI color tokens & typography.

use eframe::egui::{Color32, Context, FontId, RichText, Stroke, TextStyle, Visuals};

// Exact Discord dark tokens from spec
pub const BG_PRIMARY: Color32 = Color32::from_rgb(0x31, 0x33, 0x38); // main chat area
pub const BG_SECONDARY: Color32 = Color32::from_rgb(0x2B, 0x2D, 0x31); // left/right sidebar
pub const BG_TERTIARY: Color32 = Color32::from_rgb(0x1E, 0x1F, 0x22); // user panel, code blocks, call bar
pub const BG_FLOATING: Color32 = Color32::from_rgb(0x11, 0x12, 0x14); // modals, dropdowns, popups, toasts
pub const BG_HOVER: Color32 = Color32::from_rgb(0x35, 0x37, 0x3C); // ~rgba(255,255,255,0.05) on secondary/primary
pub const BG_ACTIVE: Color32 = Color32::from_rgb(0x40, 0x42, 0x49);
pub const MESSAGE_HOVER: Color32 = Color32::from_rgb(0x2E, 0x30, 0x35);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xF2, 0xF3, 0xF5);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xB5, 0xBA, 0xC1);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x80, 0x84, 0x8E);
pub const TEXT_LINK: Color32 = Color32::from_rgb(0x00, 0xA8, 0xFC);

pub const ACCENT: Color32 = Color32::from_rgb(0x58, 0x65, 0xF2); // Discord Blurple
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0x3B, 0x40, 0x69); // ~rgba(88,101,242,0.3) on #313338
pub const DANGER: Color32 = Color32::from_rgb(0xF2, 0x3F, 0x42);
pub const SUCCESS: Color32 = Color32::from_rgb(0x23, 0xA5, 0x59);
pub const WARNING: Color32 = Color32::from_rgb(0xF0, 0xB2, 0x32);

pub const INPUT_BG: Color32 = Color32::from_rgb(0x38, 0x3A, 0x40);
pub const INPUT_BORDER: Color32 = Color32::from_rgb(0x1E, 0x1F, 0x22);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x3B, 0x3D, 0x44);

// Backwards-compatible aliases used in auth screen
pub const SECONDARY_TEXT: Color32 = TEXT_SECONDARY;
pub const SURFACE_RAIL: Color32 = BG_TERTIARY;
pub const SURFACE_CARD: Color32 = BG_SECONDARY;
pub const CYAN_GLOW: Color32 = TEXT_LINK;

pub fn border() -> Stroke {
    Stroke::new(1.0_f32, BORDER_SUBTLE)
}

pub fn input_border() -> Stroke {
    Stroke::new(1.0_f32, INPUT_BORDER)
}

pub fn focus_border() -> Stroke {
    Stroke::new(1.25_f32, ACCENT)
}

pub fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text).color(TEXT_MUTED)
}

pub fn apply(ctx: &Context, dark_mode: bool, font_size: f32) {
    let base_font = font_size.clamp(12.0, 20.0);
    let mut visuals = if dark_mode {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    let (primary, panel, window, card, hover) = if dark_mode {
        (
            TEXT_PRIMARY,
            BG_SECONDARY,
            BG_PRIMARY,
            BG_SECONDARY,
            BG_HOVER,
        )
    } else {
        (
            Color32::from_rgb(0x06, 0x06, 0x07),
            Color32::from_rgb(0xF2, 0xF3, 0xF5),
            Color32::from_rgb(0xFF, 0xFF, 0xFF),
            Color32::from_rgb(0xEB, 0xED, 0xEF),
            Color32::from_rgb(0xDF, 0xE1, 0xE5),
        )
    };

    visuals.override_text_color = Some(primary);
    visuals.panel_fill = panel;
    visuals.window_fill = window;
    visuals.window_stroke = border();
    visuals.extreme_bg_color = INPUT_BG;
    visuals.faint_bg_color = card;
    visuals.code_bg_color = BG_TERTIARY;
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke.color = ACCENT;
    visuals.hyperlink_color = TEXT_LINK;
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
        widget.bg_stroke = Stroke::NONE;
        widget.fg_stroke.color = primary;
        widget.corner_radius = 4.0.into();
    }
    visuals.widgets.hovered.bg_fill = hover;
    visuals.widgets.active.bg_fill = ACCENT;

    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional((base_font - 2.0).max(11.0)));
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(base_font));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional((base_font - 0.5).max(12.0)));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(base_font + 6.0));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::monospace((base_font - 1.0).max(11.5)));
        style.spacing.item_spacing = eframe::egui::vec2(8.0, 4.0);
        style.spacing.scroll.bar_width = 4.0;
        style.visuals.window_corner_radius = 8.0.into();
    });
}
