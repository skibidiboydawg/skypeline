//! Original Wireline dynamic Dark & Light theme system, native OS typography,
//! and Phosphor icon font registration.

use eframe::egui::{
    self, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, RichText, Stroke,
    TextStyle, Visuals,
};
use std::sync::Arc;

/// Complete dynamic theme palette — automatically switches every surface, border,
/// bubble, and text color between Wireline Midnight (Dark) and Wireline Daylight (Light).
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub dark: bool,
    pub canvas: Color32,
    pub sidebar: Color32,
    pub surface: Color32,
    pub surface_alt: Color32,
    pub floating: Color32,
    pub hover: Color32,
    pub active: Color32,
    pub input_bg: Color32,
    pub border: Color32,
    pub hairline: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub text_link: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    #[allow(dead_code)]
    pub own_bubble: Color32,
    #[allow(dead_code)]
    pub own_bubble_text: Color32,
    #[allow(dead_code)]
    pub peer_bubble: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub call_canvas: Color32,
    pub call_tile: Color32,
}

pub const DARK: Palette = Palette {
    dark: true,
    canvas: Color32::from_rgb(0x0C, 0x0D, 0x10),
    sidebar: Color32::from_rgb(0x11, 0x12, 0x16),
    surface: Color32::from_rgb(0x17, 0x19, 0x1F),
    surface_alt: Color32::from_rgb(0x20, 0x23, 0x2B),
    floating: Color32::from_rgb(0x14, 0x16, 0x1C),
    hover: Color32::from_rgb(0x1B, 0x1E, 0x25),
    active: Color32::from_rgb(0x1D, 0x21, 0x29),
    input_bg: Color32::from_rgb(0x12, 0x14, 0x1A),
    border: Color32::from_rgb(0x27, 0x2A, 0x33),
    hairline: Color32::from_rgb(0x1E, 0x21, 0x29),
    text_primary: Color32::from_rgb(0xF4, 0xF4, 0xF6),
    text_secondary: Color32::from_rgb(0xB4, 0xB8, 0xC4),
    text_muted: Color32::from_rgb(0x84, 0x8A, 0x99),
    text_link: Color32::from_rgb(0x38, 0xBD, 0xF8),
    accent: Color32::from_rgb(0x10, 0xB9, 0x81), // Wireline Signal Emerald
    accent_soft: Color32::from_rgb(0x12, 0x2C, 0x25),
    own_bubble: Color32::from_rgb(0x10, 0xB9, 0x81),
    own_bubble_text: Color32::WHITE,
    peer_bubble: Color32::from_rgb(0x17, 0x19, 0x1F),
    success: Color32::from_rgb(0x22, 0xC5, 0x5E),
    warning: Color32::from_rgb(0xF5, 0x9E, 0x0B),
    danger: Color32::from_rgb(0xEF, 0x44, 0x44),
    call_canvas: Color32::from_rgb(0x08, 0x09, 0x0C),
    call_tile: Color32::from_rgb(0x14, 0x16, 0x1C),
};

pub const LIGHT: Palette = Palette {
    dark: false,
    canvas: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    sidebar: Color32::from_rgb(0xF7, 0xF7, 0xF8),
    surface: Color32::from_rgb(0xF1, 0xF2, 0xF5),
    surface_alt: Color32::from_rgb(0xE6, 0xE8, 0xEE),
    floating: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    hover: Color32::from_rgb(0xEC, 0xEE, 0xF2),
    active: Color32::from_rgb(0xE6, 0xF4, 0xEF),
    input_bg: Color32::from_rgb(0xF8, 0xF9, 0xFB),
    border: Color32::from_rgb(0xE2, 0xE4, 0xEA),
    hairline: Color32::from_rgb(0xEC, 0xEE, 0xF2),
    text_primary: Color32::from_rgb(0x11, 0x13, 0x18),
    text_secondary: Color32::from_rgb(0x4B, 0x51, 0x62),
    text_muted: Color32::from_rgb(0x6D, 0x73, 0x85),
    text_link: Color32::from_rgb(0x02, 0x84, 0xC7),
    accent: Color32::from_rgb(0x05, 0x96, 0x69),
    accent_soft: Color32::from_rgb(0xD1, 0xFA, 0xE5),
    own_bubble: Color32::from_rgb(0x05, 0x96, 0x69),
    own_bubble_text: Color32::WHITE,
    peer_bubble: Color32::from_rgb(0xF1, 0xF2, 0xF5),
    success: Color32::from_rgb(0x16, 0xA3, 0x4A),
    warning: Color32::from_rgb(0xD9, 0x77, 0x06),
    danger: Color32::from_rgb(0xDC, 0x26, 0x26),
    call_canvas: Color32::from_rgb(0x09, 0x0A, 0x0D),
    call_tile: Color32::from_rgb(0x17, 0x19, 0x1F),
};

impl Palette {
    pub fn border_stroke(self) -> Stroke {
        Stroke::new(1.0_f32, self.border)
    }

    pub fn hairline_stroke(self) -> Stroke {
        Stroke::new(1.0_f32, self.hairline)
    }

    pub fn focus_stroke(self) -> Stroke {
        Stroke::new(1.5_f32, self.accent)
    }

    pub fn shadow(self) -> egui::Shadow {
        egui::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(if self.dark { 130 } else { 36 }),
        }
    }
}

pub fn of(ctx: &Context) -> Palette {
    if ctx.style().visuals.dark_mode {
        DARK
    } else {
        LIGHT
    }
}

pub const ACCENT: Color32 = DARK.accent;
pub const SUCCESS: Color32 = DARK.success;
pub const WARNING: Color32 = DARK.warning;
pub const DANGER: Color32 = DARK.danger;
pub const TEXT_MUTED: Color32 = DARK.text_muted;

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

/// Registers crisp OS UI fonts (Segoe UI / system sans + SemiBold), symbol fallback,
/// and the Phosphor vector icon font (`egui-phosphor`).
pub fn setup_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    fonts.font_data.insert(
        "phosphor-fill".into(),
        egui_phosphor::Variant::Fill.font_data().into(),
    );
    fonts
        .families
        .insert(icon_family(), vec!["phosphor".into(), "phosphor-fill".into()]);

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
    let base_font = font_size.clamp(13.0, 20.0);
    let p = if dark_mode { DARK } else { LIGHT };
    let mut visuals = if dark_mode {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.override_text_color = Some(p.text_primary);
    visuals.panel_fill = p.sidebar;
    visuals.window_fill = p.floating;
    visuals.window_stroke = p.border_stroke();
    visuals.extreme_bg_color = p.input_bg;
    visuals.faint_bg_color = p.surface;
    visuals.code_bg_color = p.surface;
    visuals.selection.bg_fill = p.accent;
    visuals.selection.stroke.color = p.accent;
    visuals.hyperlink_color = p.text_link;
    visuals.popup_shadow = p.shadow();

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.bg_fill = p.surface;
        widget.weak_bg_fill = p.surface;
        widget.bg_stroke = Stroke::NONE;
        widget.fg_stroke.color = p.text_primary;
        widget.corner_radius = 8.0.into();
    }
    visuals.widgets.hovered.bg_fill = p.hover;
    visuals.widgets.active.bg_fill = p.accent;

    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional((base_font - 2.5).max(11.0)));
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(base_font));
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new((base_font - 0.5).max(12.5), semibold_family()),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(base_font + 5.0, semibold_family()),
        );
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::monospace((base_font - 1.5).max(11.5)),
        );
        style.spacing.item_spacing = egui::vec2(8.0, 4.0);
        style.spacing.scroll.bar_width = 5.0;
        style.visuals.window_corner_radius = 12.0.into();
    });
}
