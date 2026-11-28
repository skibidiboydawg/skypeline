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
    pub own_bubble: Color32,
    pub own_bubble_text: Color32,
    pub peer_bubble: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub call_canvas: Color32,
    pub call_tile: Color32,
}

pub const DARK: Palette = Palette {
    dark: true,
    canvas: Color32::from_rgb(0x0E, 0x10, 0x16),
    sidebar: Color32::from_rgb(0x14, 0x17, 0x20),
    surface: Color32::from_rgb(0x1B, 0x1F, 0x2C),
    surface_alt: Color32::from_rgb(0x23, 0x28, 0x38),
    floating: Color32::from_rgb(0x14, 0x17, 0x22),
    hover: Color32::from_rgb(0x1F, 0x24, 0x34),
    active: Color32::from_rgb(0x27, 0x2D, 0x42),
    input_bg: Color32::from_rgb(0x19, 0x1D, 0x29),
    border: Color32::from_rgb(0x26, 0x2C, 0x3E),
    hairline: Color32::from_rgb(0x1E, 0x23, 0x31),
    text_primary: Color32::from_rgb(0xF3, 0xF5, 0xFA),
    text_secondary: Color32::from_rgb(0xA6, 0xB0, 0xC6),
    text_muted: Color32::from_rgb(0x6E, 0x78, 0x91),
    text_link: Color32::from_rgb(0x60, 0xA5, 0xFA),
    accent: Color32::from_rgb(0x63, 0x66, 0xF1), // Electric Indigo
    accent_soft: Color32::from_rgb(0x24, 0x28, 0x52),
    own_bubble: Color32::from_rgb(0x4F, 0x46, 0xE5),
    own_bubble_text: Color32::WHITE,
    peer_bubble: Color32::from_rgb(0x1B, 0x1F, 0x2C),
    success: Color32::from_rgb(0x22, 0xC5, 0x5E),
    warning: Color32::from_rgb(0xF5, 0x9E, 0x0B),
    danger: Color32::from_rgb(0xEF, 0x44, 0x44),
    call_canvas: Color32::from_rgb(0x0A, 0x0C, 0x12),
    call_tile: Color32::from_rgb(0x17, 0x1B, 0x26),
};

pub const LIGHT: Palette = Palette {
    dark: false,
    canvas: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    sidebar: Color32::from_rgb(0xF4, 0xF6, 0xFA),
    surface: Color32::from_rgb(0xEE, 0xF1, 0xF7),
    surface_alt: Color32::from_rgb(0xE3, 0xE8, 0xF2),
    floating: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    hover: Color32::from_rgb(0xE9, 0xED, 0xF5),
    active: Color32::from_rgb(0xDF, 0xE5, 0xF7),
    input_bg: Color32::from_rgb(0xF1, 0xF4, 0xF9),
    border: Color32::from_rgb(0xDC, 0xE2, 0xEE),
    hairline: Color32::from_rgb(0xE5, 0xEA, 0xF2),
    text_primary: Color32::from_rgb(0x0F, 0x17, 0x2A),
    text_secondary: Color32::from_rgb(0x47, 0x55, 0x69),
    text_muted: Color32::from_rgb(0x64, 0x74, 0x8B),
    text_link: Color32::from_rgb(0x25, 0x63, 0xEB),
    accent: Color32::from_rgb(0x4F, 0x46, 0xE5),
    accent_soft: Color32::from_rgb(0xE0, 0xE7, 0xFF),
    own_bubble: Color32::from_rgb(0x4F, 0x46, 0xE5),
    own_bubble_text: Color32::WHITE,
    peer_bubble: Color32::from_rgb(0xF1, 0xF5, 0xF9),
    success: Color32::from_rgb(0x16, 0xA3, 0x4A),
    warning: Color32::from_rgb(0xD9, 0x77, 0x06),
    danger: Color32::from_rgb(0xDC, 0x26, 0x26),
    call_canvas: Color32::from_rgb(0x0F, 0x13, 0x1C),
    call_tile: Color32::from_rgb(0x1B, 0x20, 0x2E),
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
    let base_font = font_size.clamp(12.0, 20.0);
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
