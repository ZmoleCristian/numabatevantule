use egui::Color32;

use crate::domain::Severity;

const STOPS: [(f32, (u8, u8, u8)); 6] = [
    (-10.0, (56, 92, 214)),
    (0.0, (54, 160, 220)),
    (10.0, (70, 196, 150)),
    (20.0, (236, 214, 84)),
    (30.0, (240, 142, 52)),
    (40.0, (224, 58, 64)),
];

pub fn temperature(celsius: f32) -> Color32 {
    let first = STOPS[0];
    if celsius <= first.0 {
        return rgb(first.1);
    }
    let last = STOPS[STOPS.len() - 1];
    if celsius >= last.0 {
        return rgb(last.1);
    }
    for pair in STOPS.windows(2) {
        let low = pair[0];
        let high = pair[1];
        if celsius >= low.0 && celsius <= high.0 {
            let t = (celsius - low.0) / (high.0 - low.0);
            return lerp(low.1, high.1, t);
        }
    }
    rgb(last.1)
}

fn rgb(parts: (u8, u8, u8)) -> Color32 {
    Color32::from_rgb(parts.0, parts.1, parts.2)
}

fn lerp(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> Color32 {
    let mix = |x: u8, y: u8| -> u8 { (x as f32 + (y as f32 - x as f32) * t) as u8 };
    Color32::from_rgb(mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
}

pub fn severity(level: Severity) -> Color32 {
    match level {
        Severity::Green => Color32::from_rgb(70, 180, 95),
        Severity::Yellow => Color32::from_rgb(236, 206, 60),
        Severity::Orange => Color32::from_rgb(240, 142, 44),
        Severity::Red => Color32::from_rgb(224, 58, 60),
    }
}

pub fn sky_tint(icon: char) -> Color32 {
    match icon {
        '\u{f00d}' => Color32::from_rgb(255, 201, 74),
        '\u{f002}' => Color32::from_rgb(248, 224, 150),
        '\u{f013}' => Color32::from_rgb(178, 192, 211),
        '\u{f014}' => Color32::from_rgb(166, 178, 196),
        '\u{f019}' => Color32::from_rgb(96, 170, 244),
        '\u{f01b}' => Color32::from_rgb(206, 228, 255),
        '\u{f01e}' => Color32::from_rgb(190, 138, 250),
        _other => Color32::from_rgb(228, 236, 248),
    }
}

pub fn condition_icon(clouds: &str, phenomenon: &str) -> char {
    let phenomenon = phenomenon.to_lowercase();
    if phenomenon.contains("desc") || phenomenon.contains("oraj") {
        return '\u{f01e}';
    }
    if phenomenon.contains("nins") || phenomenon.contains("lapov") {
        return '\u{f01b}';
    }
    if phenomenon.contains("ploaie") || phenomenon.contains("averse") {
        return '\u{f019}';
    }
    if phenomenon.contains("cea") || phenomenon.contains("nebul") {
        return '\u{f014}';
    }
    forecast_icon(clouds)
}

pub fn forecast_icon(description: &str) -> char {
    let text = description.to_lowercase();
    if text.contains("desc") || text.contains("electric") || text.contains("oraj") {
        return '\u{f01e}';
    }
    if text.contains("nins") || text.contains("lapov") || text.contains("zapad") {
        return '\u{f01b}';
    }
    if text.contains("ploaie") || text.contains("averse") || text.contains("burni") {
        return '\u{f019}';
    }
    if text.contains("cea") || text.contains("nebul") {
        return '\u{f014}';
    }
    if text.contains("senin") {
        return '\u{f00d}';
    }
    if text.contains("variabil") || text.contains("partial") || text.contains("temporar") {
        return '\u{f002}';
    }
    if text.contains("noros") || text.contains("acoperit") {
        return '\u{f013}';
    }
    '\u{f002}'
}
