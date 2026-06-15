use egui::{Align2, Context, CornerRadius, Id, Order, Pos2, Rect, RichText, Sense, Vec2};

use crate::alertbar;
use crate::calendar;
use crate::color;
use crate::detail;
use crate::fmt;
use crate::forecastbar;
use crate::history::{DayKey, History};
use crate::theme;
use crate::viewstate::{Datasets, Overlays, Selection};

pub struct Hud<'a> {
    pub data: &'a Datasets,
    pub overlays: &'a mut Overlays,
    pub selection: &'a mut Selection,
    pub day: &'a mut usize,
    pub refresh: &'a mut bool,
    pub history: &'a mut History,
}

const DRAWER_WIDTH: f32 = 318.0;

pub fn show(ctx: &Context, hud: Hud<'_>) {
    brand(ctx);
    controls(ctx, hud.overlays, hud.refresh, hud.history);
    forecast(ctx, hud.data, hud.day);
    legend(ctx);
    corner(ctx, hud.data, hud.selection);
    if hud.history.active {
        history_bar(ctx, hud.history);
        calendar::show(ctx, hud.history);
    }
}

fn has_selection(selection: &Selection) -> bool {
    match selection {
        Selection::Nothing => false,
        Selection::Station(_name) => true,
        Selection::Warning(_index) => true,
    }
}

fn brand(ctx: &Context) {
    egui::Area::new(Id::new("hud_brand"))
        .anchor(Align2::LEFT_TOP, Vec2::new(16.0, 16.0))
        .order(Order::Foreground)
        .show(ctx, |ui| {
            theme::glass().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new('\u{f00d}'.to_string()).font(theme::weather_font(22.0)).color(theme::ACCENT));
                    ui.add_space(4.0);
                    ui.label(RichText::new("METEO ROMÂNIA").font(theme::bold_font(17.0)).color(theme::INK));
                });
            });
        });
}

fn controls(ctx: &Context, overlays: &mut Overlays, refresh: &mut bool, history: &mut History) {
    egui::Area::new(Id::new("hud_controls"))
        .anchor(Align2::RIGHT_TOP, Vec2::new(-16.0, 16.0))
        .order(Order::Foreground)
        .show(ctx, |ui| {
            theme::glass().show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("⟳").size(18.0)).clicked() {
                        *refresh = true;
                    }
                    ui.separator();
                    ui.toggle_value(&mut overlays.heat, "Căldură");
                    ui.toggle_value(&mut overlays.wind, "Vânt");
                    ui.toggle_value(&mut overlays.warnings, "Alerte");
                    ui.separator();
                    ui.toggle_value(&mut history.active, "Istoric");
                });
            });
        });
}

fn history_bar(ctx: &Context, history: &mut History) {
    egui::Area::new(Id::new("hud_history"))
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 16.0))
        .order(Order::Foreground)
        .show(ctx, |ui| {
            theme::glass().show(ui, |ui| {
                ui.set_width(460.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("⏺ Live").color(theme::ACCENT)).clicked() {
                        history.active = false;
                    }
                    ui.separator();
                    let label = format!("{} ▾", day_label(history.day));
                    if ui.button(RichText::new(label).font(theme::bold_font(13.0)).color(theme::INK)).clicked() {
                        history.calendar_open = !history.calendar_open;
                    }
                    ui.separator();
                    hour_scrubber(ui, history);
                });
            });
        });
}

fn hour_scrubber(ui: &mut egui::Ui, history: &mut History) {
    if !history.have_day {
        ui.label(RichText::new("se încarcă…").color(theme::MUTED));
        return;
    }
    let count = history.snapshots.len();
    if count == 0 {
        ui.label(RichText::new("fără date").color(theme::MUTED));
        return;
    }
    let max = count - 1;
    let mut sel = history.selected.min(max);
    if ui.add(egui::Slider::new(&mut sel, 0..=max).show_value(false)).changed() {
        history.selected = sel;
    }
    ui.add_space(8.0);
    let Some(snapshot) = history.snapshots.get(history.selected.min(max)) else {
        return;
    };
    ui.label(RichText::new(fmt::local_hm(&snapshot.timestamp)).font(theme::bold_font(13.0)).color(theme::INK));
}

fn day_label(day: DayKey) -> String {
    let index = (day.m.clamp(1, 12) - 1) as usize;
    format!("{:02} {} {}", day.d, fmt::MONTHS_SHORT_RO[index], day.y)
}

fn corner(ctx: &Context, data: &Datasets, selection: &mut Selection) {
    let show_detail = has_selection(selection);
    let show_alerts = alertbar::active(data);
    if !show_detail && !show_alerts {
        return;
    }
    egui::Area::new(Id::new("hud_corner"))
        .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-16.0, -16.0))
        .order(Order::Foreground)
        .show(ctx, |ui| {
            if show_detail {
                theme::glass().show(ui, |ui| {
                    ui.set_width(DRAWER_WIDTH);
                    ui.set_max_height(460.0);
                    detail::show(ui, data, selection);
                });
                ui.add_space(8.0);
            }
            if show_alerts {
                theme::glass().show(ui, |ui| {
                    ui.set_width(DRAWER_WIDTH);
                    ui.label(RichText::new("⚠ AVERTIZĂRI").font(theme::bold_font(12.0)).color(theme::MUTED));
                    ui.add_space(4.0);
                    alertbar::show(ui, data, selection);
                });
            }
        });
}

fn forecast(ctx: &Context, data: &Datasets, day: &mut usize) {
    egui::Area::new(Id::new("hud_forecast"))
        .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -16.0))
        .order(Order::Foreground)
        .show(ctx, |ui| {
            theme::glass().show(ui, |ui| {
                forecastbar::show(ui, data, day);
            });
        });
}

fn legend(ctx: &Context) {
    egui::Area::new(Id::new("hud_legend"))
        .anchor(Align2::LEFT_BOTTOM, Vec2::new(16.0, -16.0))
        .order(Order::Foreground)
        .show(ctx, |ui| {
            theme::glass().show(ui, |ui| {
                ui.label(RichText::new("temperatură").font(theme::bold_font(12.0)).color(theme::MUTED));
                let (rect, _response) = ui.allocate_exact_size(Vec2::new(206.0, 12.0), Sense::hover());
                let painter = ui.painter();
                let steps = 64;
                for step in 0..steps {
                    let t = step as f32 / steps as f32;
                    let celsius = -10.0 + t * 50.0;
                    let seg = Rect::from_min_size(
                        Pos2::new(rect.left() + t * rect.width(), rect.top()),
                        Vec2::new(rect.width() / steps as f32 + 1.0, rect.height()),
                    );
                    painter.rect_filled(seg, CornerRadius::ZERO, color::temperature(celsius));
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new("-10°").small().color(theme::MUTED));
                    ui.add_space(150.0);
                    ui.label(RichText::new("40°").small().color(theme::MUTED));
                });
            });
        });
}
