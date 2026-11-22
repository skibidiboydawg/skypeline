//! Skype-inspired light/dark palettes for egui.

use eframe::egui::{Color32, Context, Visuals};

pub fn apply(ctx: &Context, dark_mode: bool) {
    let mut visuals = if dark_mode {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    let accent = Color32::from_rgb(0x00, 0x68, 0xC8);
    visuals.selection.bg_fill = accent;
    visuals.selection.stroke.color = accent;
    visuals.widgets.active.bg_fill = accent;
    visuals.widgets.hovered.bg_fill = if dark_mode {
        Color32::from_rgb(0x2A, 0x4C, 0x6B)
    } else {
        Color32::from_rgb(0xD2, 0xE6, 0xFF)
    };
    visuals.hyperlink_color = accent;

    if dark_mode {
        visuals.panel_fill = Color32::from_rgb(0x14, 0x1B, 0x24);
        visuals.window_fill = Color32::from_rgb(0x1B, 0x25, 0x31);
    } else {
        visuals.panel_fill = Color32::from_rgb(0xF5, 0xF7, 0xFA);
        visuals.window_fill = Color32::from_rgb(0xFF, 0xFF, 0xFF);
    }

    ctx.set_visuals(visuals);
}
