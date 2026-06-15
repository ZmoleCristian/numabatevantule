use std::collections::BTreeMap;
use std::sync::Arc;

use egui::{Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, TextStyle, Visuals};

const INTER_REGULAR: &[u8] = include_bytes!("../assets/inter-regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../assets/inter-bold.ttf");
const WEATHER: &[u8] = include_bytes!("../assets/weathericons.ttf");

pub const ACCENT: Color32 = Color32::from_rgb(86, 156, 255);
pub const PANEL: Color32 = Color32::from_rgb(15, 19, 28);
pub const CARD: Color32 = Color32::from_rgb(23, 29, 42);
pub const INK: Color32 = Color32::from_rgb(232, 238, 248);
pub const MUTED: Color32 = Color32::from_rgb(140, 152, 172);

pub fn glass() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(18, 24, 38, 214))
        .corner_radius(CornerRadius::same(16))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_white_alpha(28)))
        .inner_margin(egui::Margin::same(12))
        .shadow(egui::Shadow {
            offset: [0, 10],
            blur: 28,
            spread: 0,
            color: Color32::from_black_alpha(130),
        })
}

pub fn weather_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("weather".into()))
}

pub fn bold_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

pub fn install(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("inter".to_owned(), Arc::new(FontData::from_static(INTER_REGULAR)));
    fonts
        .font_data
        .insert("bold".to_owned(), Arc::new(FontData::from_static(INTER_BOLD)));
    fonts
        .font_data
        .insert("weather".to_owned(), Arc::new(FontData::from_static(WEATHER)));

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter".to_owned());
    fonts
        .families
        .insert(FontFamily::Name("bold".into()), vec!["bold".to_owned()]);
    fonts
        .families
        .insert(FontFamily::Name("weather".into()), vec!["weather".to_owned()]);
    ctx.set_fonts(fonts);

    let mut style = (*ctx.global_style()).clone();
    style.text_styles = text_styles();
    apply_visuals(&mut style.visuals);
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    ctx.set_global_style(style);
}

fn text_styles() -> BTreeMap<TextStyle, FontId> {
    [
        (TextStyle::Heading, bold_font(26.0)),
        (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
    ]
    .into_iter()
    .collect()
}

fn apply_visuals(visuals: &mut Visuals) {
    visuals.panel_fill = PANEL;
    visuals.window_fill = CARD;
    visuals.faint_bg_color = Color32::from_rgb(28, 35, 50);
    visuals.extreme_bg_color = Color32::from_rgb(9, 12, 18);
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = Color32::from_rgb(38, 78, 130);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(8);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
    visuals.widgets.active.corner_radius = CornerRadius::same(8);
}
