use egui::{Align2, Color32, CornerRadius, Sense, Stroke, StrokeKind, Vec2};

use crate::color;
use crate::domain::Reading;
use crate::fmt;
use crate::theme;
use crate::viewstate::{Datasets, Remote};

const CARD_SIZE: Vec2 = Vec2::new(104.0, 84.0);

struct DayCard {
    label: String,
    hi: Reading,
    lo: Reading,
    icon: char,
}

pub fn show(ui: &mut egui::Ui, data: &Datasets, day: &mut usize) {
    ui.horizontal(|ui| {
        if today_card(ui, data, *day == 0) {
            *day = 0;
        }
        let cards = forecast_cards(data);
        for (index, card) in cards.iter().enumerate() {
            let slot = index + 1;
            if draw_card(ui, &card.label, card.icon, card.hi, card.lo, *day == slot) {
                *day = slot;
            }
        }
    });
}

fn today_card(ui: &mut egui::Ui, data: &Datasets, selected: bool) -> bool {
    let hi = live_peak(data);
    let label = today_label(data);
    draw_card(ui, &label, '\u{f00d}', hi, Reading::Missing, selected)
}

fn today_label(data: &Datasets) -> String {
    let cities = match &data.forecasts {
        Remote::Ready(list) => list,
        Remote::Loading => return "Azi".to_string(),
        Remote::Failed(_cause) => return "Azi".to_string(),
    };
    let Some(city) = cities.first() else {
        return "Azi".to_string();
    };
    let Some(first_day) = city.days.first() else {
        return "Azi".to_string();
    };
    format!("Azi {}", fmt::day_before(&first_day.date))
}

fn draw_card(ui: &mut egui::Ui, label: &str, icon: char, hi: Reading, lo: Reading, selected: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(CARD_SIZE, Sense::click());
    let painter = ui.painter();

    let background = if selected {
        Color32::from_rgb(34, 52, 84)
    } else if response.hovered() {
        Color32::from_rgb(28, 35, 50)
    } else {
        theme::CARD
    };
    painter.rect_filled(rect, CornerRadius::same(12), background);
    if selected {
        painter.rect_stroke(
            rect,
            CornerRadius::same(12),
            Stroke::new(1.6_f32, theme::ACCENT),
            StrokeKind::Inside,
        );
    }

    painter.text(
        rect.center_top() + Vec2::new(0.0, 13.0),
        Align2::CENTER_CENTER,
        label,
        theme::bold_font(12.0),
        theme::INK,
    );
    painter.text(
        rect.center() + Vec2::new(0.0, 0.0),
        Align2::CENTER_CENTER,
        icon.to_string(),
        theme::weather_font(26.0),
        color::sky_tint(icon),
    );
    let temps = match lo {
        Reading::Missing => fmt::reading_short(hi),
        Reading::Celsius(_value) => format!("{}  {}", fmt::reading_short(hi), fmt::reading_short(lo)),
    };
    painter.text(
        rect.center_bottom() + Vec2::new(0.0, -13.0),
        Align2::CENTER_CENTER,
        temps,
        theme::bold_font(13.0),
        hi_color(hi),
    );

    response.clicked()
}

fn hi_color(reading: Reading) -> Color32 {
    match reading {
        Reading::Celsius(value) => color::temperature(value),
        Reading::Missing => theme::MUTED,
    }
}

fn live_peak(data: &Datasets) -> Reading {
    let stations = match &data.stations {
        Remote::Ready(list) => list,
        Remote::Loading => return Reading::Missing,
        Remote::Failed(_cause) => return Reading::Missing,
    };
    let mut peak = f32::MIN;
    for station in stations {
        match station.temp {
            Reading::Celsius(value) => {
                if value > peak {
                    peak = value;
                }
            }
            Reading::Missing => {}
        }
    }
    if peak == f32::MIN {
        return Reading::Missing;
    }
    Reading::Celsius(peak)
}

fn forecast_cards(data: &Datasets) -> Vec<DayCard> {
    let cities = match &data.forecasts {
        Remote::Ready(list) => list,
        Remote::Loading => return Vec::new(),
        Remote::Failed(_cause) => return Vec::new(),
    };
    let mut span = usize::MAX;
    for city in cities {
        if city.days.len() < span {
            span = city.days.len();
        }
    }
    if span == usize::MAX {
        return Vec::new();
    }

    let mut cards = Vec::with_capacity(span);
    for index in 0..span {
        let mut hi = f32::MIN;
        let mut lo = f32::MAX;
        let mut rank = 0u8;
        let mut icon = '\u{f002}';
        let mut label = String::new();
        for city in cities {
            let day = &city.days[index];
            label = format!("{} {}", fmt::weekday_ro(&day.date), fmt::pretty_date(&day.date));
            match day.tmax_value {
                Reading::Celsius(value) => {
                    if value > hi {
                        hi = value;
                    }
                }
                Reading::Missing => {}
            }
            match day.tmin_value {
                Reading::Celsius(value) => {
                    if value < lo {
                        lo = value;
                    }
                }
                Reading::Missing => {}
            }
            let weight = rank_of(&day.description);
            if weight >= rank {
                rank = weight;
                icon = color::forecast_icon(&day.description);
            }
        }
        cards.push(DayCard {
            label,
            hi: peak_reading(hi, f32::MIN),
            lo: peak_reading(lo, f32::MAX),
            icon,
        });
    }
    cards
}

fn peak_reading(value: f32, sentinel: f32) -> Reading {
    if value == sentinel {
        return Reading::Missing;
    }
    Reading::Celsius(value)
}

fn rank_of(description: &str) -> u8 {
    let text = description.to_lowercase();
    if text.contains("desc") || text.contains("electric") || text.contains("oraj") {
        return 6;
    }
    if text.contains("nins") || text.contains("lapov") || text.contains("zapad") {
        return 5;
    }
    if text.contains("ploaie") || text.contains("averse") || text.contains("burni") {
        return 4;
    }
    if text.contains("cea") || text.contains("nebul") {
        return 3;
    }
    if text.contains("noros") || text.contains("acoperit") {
        return 2;
    }
    if text.contains("variabil") || text.contains("partial") || text.contains("temporar") {
        return 1;
    }
    0
}
