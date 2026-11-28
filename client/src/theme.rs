//! Authentic Discord dark UI palette, native OS typography (Segoe UI / system sans),
//! and Phosphor icon font registration.

use eframe::egui::{
    self, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, RichText, Stroke,
    TextStyle, Visuals,
};
use std::sync::Arc;

// Exact Discord dark tokens
pub const BG_PRIMARY: Color32 = Color32::from_rgb(0x31, 0x33, 0x38); // main chat area
pub const BG_SECONDARY: Color32 = Color32::from_rgb(0x2B, 0x2D, 0x31); // left sidebar
pub const BG_USER_PANEL: Color32 = Color32::from_rgb(0x23, 0x24, 0x28); // bottom user dock & voice bar
pub const BG_TERTIARY: Color32 = Color32::from_rgb(0x1E, 0x1F, 0x22); // darkest canvas / inputs / code blocks
pub const BG_FLOATING: Color32 = Color32::from_rgb(0x11, 0x12, 0x14); // modals, popups, menus, toasts
pub const BG_HOVER: Color32 = Color32::from_rgb(0x35, 0x37, 0x3C);
pub const BG_ACTIVE: Color32 = Color32::from_rgb(0x40, 0x42, 0x49);
pub const MESSAGE_HOVER: Color32 = Color32::from_rgb(0x2E, 0x30, 0x35);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xF2, 0xF3, 0xF5);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xB5, 0xBA, 0xC1);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x94, 0x9B, 0xA4);
pub const TEXT_LINK: Color32 = Color32::from_rgb(0x00, 0xA8, 0xFC);

pub const ACCENT: Color32 = Color32::from_rgb(0x58, 0x65, 0xF2); // Discord Blurple
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0x3B, 0x40, 0x69);
pub const DANGER: Color32 = Color32::from_rgb(0xDA, 0x37, 0x3C); // authentic Discord red
pub const SUCCESS: Color32 = Color32::from_rgb(0x23, 0xA5, 0x59); // online green
pub const WARNING: Color32 = Color32::from_rgb(0xF0, 0xB2, 0x32); // idle amber

pub const INPUT_BG: Color32 = Color32::from_rgb(0x38, 0x3A, 0x40);
pub const INPUT_BORDER: Color32 = Color32::from_rgb(0x1E, 0x1F, 0x22);
pub const HEADER_HAIRLINE: Color32 = Color32::from_rgb(0x22, 0x24, 0x28);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x3F, 0x41, 0x47);

pub fn border() -> Stroke {
    Stroke::new(1.0_f32, BORDER_SUBTLE)
}

pub fn hairline() -> Stroke {
    Stroke::new(1.0_f32, HEADER_HAIRLINE)
}

pub fn input_border() -> Stroke {
    Stroke::new(1.0_f32, INPUT_BORDER)
}

pub fn focus_border() -> Stroke {
    Stroke::new(1.25_f32, ACCENT)
}

pub fn semibold_family() -> FontFamily {
    FontFamily::Name("semibold".into())
}

pub fn icon_family() -> FontFamily {
    FontFamily::Name("icons".into())
}

pub fn semibold(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).font(FontId::new(size, semibold_family()))
}

fn load_first_font(paths: &[&str]) -> Option<Arc<FontData>> {
    for path in paths {
        if let Ok(bytes) = std::fs::read(path) {
            return Some(Arc::new(FontData::from_owned(bytes)));
        }
    }
    None
}

/// Registers crisp OS UI fonts (Segoe UI / system sans + SemiBold), emoji symbol fallback,
/// and the Phosphor vector icon font (`egui-phosphor`).
pub fn setup_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    // 1. Phosphor crisp vector icon font
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    fonts.font_data.insert(
        "phosphor-fill".into(),
        egui_phosphor::Variant::Fill.font_data().into(),
    );
    fonts
        .families
        .insert(icon_family(), vec!["phosphor".into(), "phosphor-fill".into()]);

    // 2. Native clean sans-serif UI font (replaces egui's thin Ubuntu-Light default)
    #[cfg(target_os = "windows")]
    let regular_paths: &[&str] = &[
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];
    #[cfg(not(target_os = "windows"))]
    let regular_paths: &[&str] = &[
        "/usr/share/fonts/TTF/Inter-Regular.ttf",
        "/usr/share/fonts/inter/Inter-Regular.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];

    #[cfg(target_os = "windows")]
    let semibold_paths: &[&str] = &[
        "C:\\Windows\\Fonts\\seguisb.ttf",
        "C:\\Windows\\Fonts\\segoeuib.ttf",
    ];
    #[cfg(not(target_os = "windows"))]
    let semibold_paths: &[&str] = &[
        "/usr/share/fonts/TTF/Inter-SemiBold.ttf",
        "/usr/share/fonts/inter/Inter-SemiBold.ttf",
        "/usr/share/fonts/noto/NotoSans-SemiBold.ttf",
        "/usr/share/fonts/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    ];

    if let Some(reg) = load_first_font(regular_paths) {
        fonts.font_data.insert("ui-regular".into(), reg);
        if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
            prop.insert(0, "ui-regular".into());
        }
    }

    let mut sb_list = Vec::new();
    if let Some(sb) = load_first_font(semibold_paths) {
        fonts.font_data.insert("ui-semibold".into(), sb);
        sb_list.push("ui-semibold".into());
    }
    if fonts.font_data.contains_key("ui-regular") {
        sb_list.push("ui-regular".into());
    }
    if let Some(prop) = fonts.families.get(&FontFamily::Proportional) {
        for f in prop {
            if !sb_list.contains(f) {
                sb_list.push(f.clone());
            }
        }
    }
    fonts.families.insert(semibold_family(), sb_list);

    // 3. Clean symbol fallback on Windows so emoji/symbols never render as boxes
    #[cfg(target_os = "windows")]
    if let Some(sym) = load_first_font(&["C:\\Windows\\Fonts\\seguisym.ttf"]) {
        fonts.font_data.insert("ui-symbols".into(), sym);
        if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
            prop.push("ui-symbols".into());
        }
    }

    ctx.set_fonts(fonts);
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
    visuals.window_stroke = Stroke::new(1.0_f32, HEADER_HAIRLINE);
    visuals.extreme_bg_color = INPUT_BG;
    visuals.faint_bg_color = card;
    visuals.code_bg_color = BG_TERTIARY;
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke.color = ACCENT;
    visuals.hyperlink_color = TEXT_LINK;
    visuals.widgets.hovered.bg_fill = hover;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.open.bg_fill = card;
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };

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
            .insert(TextStyle::Small, FontId::proportional((base_font - 2.5).max(11.0)));
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(base_font));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::new((base_font - 0.5).max(12.5), semibold_family()));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::new(base_font + 5.0, semibold_family()));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::monospace((base_font - 1.5).max(11.5)));
        style.spacing.item_spacing = egui::vec2(8.0, 4.0);
        style.spacing.scroll.bar_width = 5.0;
        style.visuals.window_corner_radius = 8.0.into();
    });
}
