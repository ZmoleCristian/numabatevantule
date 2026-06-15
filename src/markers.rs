use egui::{Align2, Color32, CornerRadius, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

use crate::color;
use crate::domain::Reading;
use crate::theme;

pub struct MapPoint {
    pub pos: Pos2,
    pub name: String,
    pub reading: Reading,
    pub icon: char,
}

const MAJORS: [&str; 40] = [
    "bucuresti", "cluj", "timisoara", "iasi", "constanta", "craiova", "brasov", "galati",
    "ploiesti", "oradea", "braila", "arad", "pitesti", "sibiu", "bacau", "targu mures",
    "baia mare", "buzau", "satu mare", "botosani", "ramnicu valcea", "suceava", "piatra neamt",
    "drobeta", "focsani", "tulcea", "resita", "slatina", "calarasi", "alba iulia", "giurgiu",
    "deva", "bistrita", "targoviste", "zalau", "sfantu gheorghe", "slobozia", "alexandria",
    "miercurea ciuc", "vaslui",
];

pub fn draw(painter: &Painter, points: &[MapPoint], selected: &str, zoom: f32) {
    let mut placed: Vec<Rect> = Vec::new();

    for point in points {
        if point.name == selected {
            draw_card(painter, point, true);
            placed.push(card_rect(point.pos));
        }
    }

    for point in points {
        if point.name == selected {
            continue;
        }
        let slot = card_rect(point.pos);
        if placed.iter().any(|taken| taken.intersects(slot)) {
            draw_dot(painter, point);
            continue;
        }
        draw_card(painter, point, false);
        placed.push(slot);
    }

    let show_minor = zoom >= 2.2;
    let mut name_slots: Vec<Rect> = Vec::new();
    for major_pass in [true, false] {
        if !major_pass && !show_minor {
            continue;
        }
        for point in points {
            if is_major(&point.name) != major_pass {
                continue;
            }
            let size = if major_pass { 12.0 } else { 10.5 };
            let slot = name_rect(painter, point.pos, &point.name, size);
            if name_slots.iter().any(|taken| taken.intersects(slot)) {
                continue;
            }
            draw_name(painter, slot, &point.name, size);
            name_slots.push(slot);
        }
    }
}

fn normalize(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.to_lowercase().chars() {
        let mapped = match ch {
            'ă' | 'â' => 'a',
            'î' => 'i',
            'ș' | 'ş' => 's',
            'ț' | 'ţ' => 't',
            other => other,
        };
        out.push(mapped);
    }
    out
}

fn is_major(name: &str) -> bool {
    let key = normalize(name);
    MAJORS.iter().any(|city| key.contains(city))
}

fn name_rect(painter: &Painter, pos: Pos2, name: &str, size: f32) -> Rect {
    let galley = painter.layout_no_wrap(name.to_string(), theme::bold_font(size), Color32::WHITE);
    Rect::from_center_size(pos + Vec2::new(0.0, 36.0), galley.size() + Vec2::new(10.0, 5.0))
}

fn draw_name(painter: &Painter, slot: Rect, name: &str, size: f32) {
    painter.rect_filled(slot, CornerRadius::same(5), Color32::from_black_alpha(170));
    painter.text(
        slot.center(),
        Align2::CENTER_CENTER,
        name,
        theme::bold_font(size),
        Color32::from_rgb(238, 244, 252),
    );
}

fn card_rect(pos: Pos2) -> Rect {
    Rect::from_center_size(pos + Vec2::new(0.0, -8.0), Vec2::new(62.0, 74.0))
}

fn fill_for(reading: Reading) -> Color32 {
    match reading {
        Reading::Celsius(value) => color::temperature(value),
        Reading::Missing => Color32::from_gray(120),
    }
}

fn draw_card(painter: &Painter, point: &MapPoint, selected: bool) {
    let tint = fill_for(point.reading);
    let icon_size = if selected { 42.0 } else { 34.0 };
    let icon_pos = point.pos + Vec2::new(0.0, -24.0);

    painter.text(
        icon_pos + Vec2::new(1.5, 2.0),
        Align2::CENTER_CENTER,
        point.icon.to_string(),
        theme::weather_font(icon_size),
        Color32::from_black_alpha(150),
    );
    painter.text(
        icon_pos,
        Align2::CENTER_CENTER,
        point.icon.to_string(),
        theme::weather_font(icon_size),
        color::sky_tint(point.icon),
    );

    let dot_radius = if selected { 6.0 } else { 4.5 };
    painter.circle_filled(point.pos, dot_radius + 1.5, Color32::from_black_alpha(130));
    painter.circle_filled(point.pos, dot_radius, tint);
    let ring = if selected {
        Stroke::new(2.5_f32, theme::ACCENT)
    } else {
        Stroke::new(1.5_f32, Color32::WHITE)
    };
    painter.circle_stroke(point.pos, dot_radius, ring);

    draw_pill(painter, point.pos + Vec2::new(0.0, 20.0), point.reading, tint, selected);
}

fn draw_pill(painter: &Painter, center: Pos2, reading: Reading, tint: Color32, selected: bool) {
    let label = match reading {
        Reading::Celsius(value) => format!("{value:.0}°"),
        Reading::Missing => "—".to_string(),
    };
    let font = theme::bold_font(if selected { 17.0 } else { 15.0 });
    let galley = painter.layout_no_wrap(label, font, Color32::WHITE);
    let pad = Vec2::new(8.0, 3.0);
    let rect = Rect::from_center_size(center, galley.size() + pad * 2.0);
    painter.rect_filled(rect, CornerRadius::same(7), Color32::from_black_alpha(190));
    painter.rect_stroke(rect, CornerRadius::same(7), Stroke::new(1.5_f32, tint), StrokeKind::Inside);
    painter.galley(rect.center() - galley.size() * 0.5, galley, Color32::WHITE);
}

fn draw_dot(painter: &Painter, point: &MapPoint) {
    let tint = fill_for(point.reading);
    painter.circle_filled(point.pos, 4.5, Color32::from_black_alpha(120));
    painter.circle_filled(point.pos, 3.5, tint);
    painter.circle_stroke(point.pos, 3.5, Stroke::new(1.0_f32, Color32::from_white_alpha(170)));
}

pub fn hit_test(points: &[MapPoint], cursor: Pos2) -> usize {
    let mut best_index = points.len();
    let mut best_distance = f32::MAX;
    for (index, point) in points.iter().enumerate() {
        let distance = point.pos.distance(cursor);
        if distance < best_distance {
            best_distance = distance;
            best_index = index;
        }
    }
    if best_index < points.len() && best_distance <= 24.0 {
        return best_index;
    }
    points.len()
}
