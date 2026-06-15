use egui::{Align2, Context, Id, Order, RichText, Vec2};

use crate::fmt;
use crate::history::{DayKey, History};
use crate::theme;

const WEEKDAY_INITIALS: [&str; 7] = ["L", "M", "M", "J", "V", "S", "D"];

pub fn show(ctx: &Context, history: &mut History) {
    if !history.calendar_open || !history.have_newest {
        return;
    }
    egui::Area::new(Id::new("hud_calendar"))
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 92.0))
        .order(Order::Foreground)
        .show(ctx, |ui| {
            theme::glass().show(ui, |ui| {
                ui.set_width(248.0);
                header(ui, history);
                ui.add_space(6.0);
                weekdays(ui);
                grid(ui, history);
            });
        });
}

fn header(ui: &mut egui::Ui, history: &mut History) {
    ui.horizontal(|ui| {
        if ui.button(RichText::new("◀").size(16.0)).clicked() {
            history.view = prev_month(history.view);
        }
        let title = format!("{} {}", fmt::MONTHS_RO[month_index(history.view.m)], history.view.y);
        ui.add_space(8.0);
        ui.label(RichText::new(title).font(theme::bold_font(14.0)).color(theme::INK));
        ui.add_space(8.0);
        if ui.button(RichText::new("▶").size(16.0)).clicked() {
            history.view = next_month(history.view);
        }
    });
}

fn weekdays(ui: &mut egui::Ui) {
    egui::Grid::new("cal_head").min_col_width(30.0).show(ui, |ui| {
        for label in WEEKDAY_INITIALS {
            ui.label(RichText::new(label).font(theme::bold_font(11.0)).color(theme::MUTED));
        }
        ui.end_row();
    });
}

fn grid(ui: &mut egui::Ui, history: &mut History) {
    let view = history.view;
    let lead = fmt::weekday_mon0(view.y, view.m, 1) as i64;
    let dim = fmt::days_in_month(view.y, view.m);
    egui::Grid::new("cal_grid").min_col_width(30.0).show(ui, |ui| {
        for cell in 0..42usize {
            let number = cell as i64 - lead + 1;
            if number < 1 || number > dim {
                ui.label(RichText::new("  ").monospace());
            } else {
                day_cell(ui, history, DayKey { y: view.y, m: view.m, d: number });
            }
            if cell % 7 == 6 {
                ui.end_row();
            }
        }
    });
}

fn day_cell(ui: &mut egui::Ui, history: &mut History, key: DayKey) {
    let future = key > history.newest;
    let selected = key == history.day;
    let text = RichText::new(format!("{:>2}", key.d)).monospace();
    let response = ui.add_enabled(!future, egui::Button::selectable(selected, text));
    if response.clicked() {
        history.day = key;
        history.view = key;
        history.have_day = false;
        history.calendar_open = false;
    }
}

fn month_index(month: i64) -> usize {
    (month.clamp(1, 12) - 1) as usize
}

fn prev_month(view: DayKey) -> DayKey {
    if view.m <= 1 {
        DayKey { y: view.y - 1, m: 12, d: 1 }
    } else {
        DayKey { y: view.y, m: view.m - 1, d: 1 }
    }
}

fn next_month(view: DayKey) -> DayKey {
    if view.m >= 12 {
        DayKey { y: view.y + 1, m: 1, d: 1 }
    } else {
        DayKey { y: view.y, m: view.m + 1, d: 1 }
    }
}
