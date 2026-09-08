use std::fmt::Display;

use crate::state::sections::{PrimaryChoice, SecondaryChoice, Summary};

pub(super) fn preview_slice<T: Display>(values: &[T]) -> String {
    let preview: Vec<String> = values.iter().take(6).map(ToString::to_string).collect();
    if values.len() > 6 {
        format!("[{}, ...]", preview.join(", "))
    } else {
        format!("[{}]", preview.join(", "))
    }
}

pub(super) fn preview_f32_slice(values: &[f32]) -> String {
    let preview: Vec<String> = values
        .iter()
        .take(6)
        .map(|value| format!("{value:.2}"))
        .collect();
    if values.len() > 6 {
        format!("[{}, ...]", preview.join(", "))
    } else {
        format!("[{}]", preview.join(", "))
    }
}

pub(super) fn show_multi_data_preview(ui: &mut egui::Ui, items: &[(u32, usize, String)]) {
    if items.is_empty() {
        ui.label("no indices");
    } else {
        for (index, len, preview) in items {
            ui.label(format!("[{index}] len = {len}, preview = {preview}"));
        }
    }
}

pub(super) fn show_primary_choice_selector(ui: &mut egui::Ui, value: &mut PrimaryChoice) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui.selectable_value(value, PrimaryChoice::A, "A").changed();
        changed |= ui.selectable_value(value, PrimaryChoice::B, "B").changed();
        changed |= ui.selectable_value(value, PrimaryChoice::C, "C").changed();
    });
    changed
}

pub(super) fn show_secondary_choice_selector(
    ui: &mut egui::Ui,
    value: &mut SecondaryChoice,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui
            .selectable_value(value, SecondaryChoice::X, "X")
            .changed();
        changed |= ui
            .selectable_value(value, SecondaryChoice::Y, "Y")
            .changed();
        changed |= ui
            .selectable_value(value, SecondaryChoice::Z, "Z")
            .changed();
    });
    changed
}

pub(super) fn primary_choice_label(value: PrimaryChoice) -> &'static str {
    match value {
        PrimaryChoice::A => "A",
        PrimaryChoice::B => "B",
        PrimaryChoice::C => "C",
    }
}

pub(super) fn format_summary(value: &Summary) -> String {
    format!(
        "enabled = {}, level = {}, name = {}",
        value.enabled, value.level, value.name
    )
}
