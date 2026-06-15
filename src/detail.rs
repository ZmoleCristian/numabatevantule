use egui::{Color32, RichText};

use crate::color;
use crate::domain::Reading;
use crate::fmt;
use crate::theme;
use crate::viewstate::{Datasets, Remote, Selection};

pub fn show(ui: &mut egui::Ui, data: &Datasets, selection: &Selection) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(6.0);
        match selection {
            Selection::Nothing => {
                ui.label(RichText::new("Alege o stație sau o avertizare.").color(theme::MUTED));
            }
            Selection::Station(name) => station(ui, data, name),
            Selection::Warning(index) => warning(ui, data, *index),
        }
    });
}

fn station(ui: &mut egui::Ui, data: &Datasets, name: &str) {
    let stations = match &data.stations {
        Remote::Ready(list) => list,
        Remote::Loading => return,
        Remote::Failed(_cause) => return,
    };
    let Some(station) = stations.iter().find(|item| item.name == name) else {
        return;
    };

    ui.horizontal(|ui| {
        let icon = color::condition_icon(&station.clouds, &station.phenomenon);
        ui.label(RichText::new(icon.to_string()).font(theme::weather_font(48.0)).color(color::sky_tint(icon)));
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.label(RichText::new(&station.name).font(theme::bold_font(22.0)).color(theme::INK));
            ui.label(
                RichText::new(fmt::reading_text(station.temp))
                    .font(theme::bold_font(30.0))
                    .color(temp_color(station.temp)),
            );
        });
    });
    ui.add_space(8.0);

    metric(ui, "umiditate", &station.humidity);
    metric(ui, "vânt", &station.wind);
    metric(ui, "nori", &station.clouds);
    metric(ui, "presiune", &station.pressure);
    if station.phenomenon != "indisponibil" {
        metric(ui, "fenomen", &station.phenomenon);
    }
    if station.snow != "indisponibil" {
        metric(ui, "zăpadă", &station.snow);
    }
    if station.water != "indisponibil" {
        metric(ui, "temp. apă", &station.water);
    }
    metric(ui, "actualizat", &station.updated);

    forecast_strip(ui, data, name);
}

fn forecast_strip(ui: &mut egui::Ui, data: &Datasets, name: &str) {
    let forecasts = match &data.forecasts {
        Remote::Ready(list) => list,
        Remote::Loading => return,
        Remote::Failed(_cause) => return,
    };
    let Some(city) = forecasts.iter().find(|item| item.name.eq_ignore_ascii_case(name)) else {
        return;
    };
    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("Prognoză 5 zile").font(theme::bold_font(16.0)).color(theme::INK));
    for entry in &city.days {
        ui.horizontal(|ui| {
            let day_icon = color::forecast_icon(&entry.description);
            ui.label(
                RichText::new(day_icon.to_string())
                    .font(theme::weather_font(22.0))
                    .color(color::sky_tint(day_icon)),
            );
            ui.label(RichText::new(format!("{} {}", fmt::weekday_ro(&entry.date), fmt::pretty_date(&entry.date))).monospace().color(theme::MUTED));
            ui.label(format!("{}° / {}°", entry.tmin, entry.tmax_text));
            ui.label(RichText::new(&entry.description).color(theme::MUTED));
        });
    }
}

fn warning(ui: &mut egui::Ui, data: &Datasets, index: usize) {
    let list = match &data.alerts {
        Remote::Ready(list) => list,
        Remote::Loading => return,
        Remote::Failed(_cause) => return,
    };
    let Some(alert) = list.get(index) else {
        return;
    };
    let tint = color::severity(alert.severity);

    ui.label(RichText::new(&alert.kind).font(theme::bold_font(20.0)).color(tint));
    ui.separator();
    metric(ui, "început", &alert.appeared);
    metric(ui, "expiră", &alert.expires);
    metric(ui, "interval", &alert.interval);
    if !alert.phenomena.is_empty() {
        metric(ui, "fenomene", &alert.phenomena);
    }
    if !alert.zone_text.is_empty() {
        metric(ui, "zonă", &alert.zone_text);
    }
    if !alert.counties.is_empty() {
        let names: Vec<&str> = alert.counties.iter().map(|hit| hit.name.as_str()).collect();
        metric(ui, "județe", &names.join(", "));
    }
    if !alert.message.is_empty() {
        ui.add_space(6.0);
        ui.separator();
        ui.label(&alert.message);
    }
}

fn metric(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.add_space(3.0);
    ui.label(RichText::new(key.to_uppercase()).font(theme::bold_font(10.0)).color(theme::MUTED));
    ui.label(RichText::new(value).color(theme::INK));
}

fn temp_color(reading: Reading) -> Color32 {
    match reading {
        Reading::Celsius(value) => color::temperature(value),
        Reading::Missing => theme::MUTED,
    }
}
