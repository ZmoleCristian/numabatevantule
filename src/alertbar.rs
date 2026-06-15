use egui::RichText;

use crate::color;
use crate::domain::{Alert, Severity};
use crate::viewstate::{Datasets, Remote, Selection};

pub fn active(data: &Datasets) -> bool {
    !collect(&data.alerts).is_empty()
}

pub fn show(ui: &mut egui::Ui, data: &Datasets, selection: &mut Selection) {
    let general = collect(&data.alerts);
    ui.horizontal_wrapped(|ui| {
        for (index, label, severity) in &general {
            let active = is_selected(selection, *index);
            let chip = RichText::new(format!("⚠ {label}")).color(color::severity(*severity));
            if ui.selectable_label(active, chip).clicked() {
                *selection = Selection::Warning(*index);
            }
        }
    });
}

fn collect(remote: &Remote<Vec<Alert>>) -> Vec<(usize, String, Severity)> {
    match remote {
        Remote::Ready(list) => list
            .iter()
            .enumerate()
            .map(|(index, alert)| (index, format!("{} · {}", alert.kind, alert.interval), alert.severity))
            .collect(),
        Remote::Loading => Vec::new(),
        Remote::Failed(_cause) => Vec::new(),
    }
}

fn is_selected(selection: &Selection, index: usize) -> bool {
    match selection {
        Selection::Warning(current_index) => *current_index == index,
        Selection::Nothing => false,
        Selection::Station(_name) => false,
    }
}
