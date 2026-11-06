//! Reusable widgets shared across pages.

use super::theme::Palette;
use crate::enum_macro::KeyEnum;
use crate::model::{Account, CategoryTree, Money};
use chrono::NaiveDate;
use egui::{ComboBox, Id, Margin, Response, RichText, Sense, TextEdit, Ui};

/// A small "ℹ" marker that shows `text` on hover.
pub fn help(ui: &mut Ui, text: &str) -> Response {
    ui.add(egui::Label::new(RichText::new("ℹ").weak()).sense(Sense::hover())).on_hover_text(text)
}

/// Page heading with an optional help marker.
pub fn heading(ui: &mut Ui, title: &str, help_text: &str) {
    ui.horizontal(|ui| {
        ui.heading(title);
        if !help_text.is_empty() {
            help(ui, help_text);
        }
    });
    ui.add_space(4.0);
}

/// Label followed by a help marker (for form rows).
pub fn label_with_help(ui: &mut Ui, label: &str, help_text: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        help(ui, help_text);
    });
}

/// Combo box over every variant of a string-keyed enum.
pub fn enum_combo<T: KeyEnum + PartialEq>(ui: &mut Ui, id: impl std::hash::Hash, value: &mut T) -> bool {
    let mut changed = false;
    ComboBox::from_id_salt(id).selected_text(KeyEnum::label(*value)).show_ui(ui, |ui| {
        for &v in T::all() {
            changed |= ui.selectable_value(value, v, KeyEnum::label(v)).changed();
        }
    });
    changed
}

/// Category picker with a type-to-filter box. `none_label` adds a "no category" entry when non-empty.
pub fn category_combo(ui: &mut Ui, id: Id, value: &mut Option<i64>, cats: &CategoryTree, none_label: &str, width: f32) -> bool {
    let selected = value.map_or_else(|| none_label.to_string(), |c| cats.full_name(c));
    category_combo_with_text(ui, id, value, cats, none_label, width, RichText::new(selected))
}

/// Like [`category_combo`] but with custom selected text.
pub fn category_combo_with_text(
    ui: &mut Ui,
    id: Id,
    value: &mut Option<i64>,
    cats: &CategoryTree,
    none_label: &str,
    width: f32,
    selected_text: RichText,
) -> bool {
    let mut changed = false;
    let filter_id = id.with("filter");
    ComboBox::from_id_salt(id).selected_text(selected_text).width(width).height(420.0).show_ui(ui, |ui| {
        let mut filter: String = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();
        ui.add(TextEdit::singleline(&mut filter).hint_text("Type to filter…").desired_width(200.0));
        ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
        let needle = filter.to_lowercase();
        if !none_label.is_empty() && needle.is_empty() {
            changed |= ui.selectable_value(value, None, RichText::new(none_label).italics()).changed();
        }
        for c in cats.ordered() {
            if !needle.is_empty() && !cats.full_name(c.id).to_lowercase().contains(&needle) {
                continue;
            }
            let text =
                if c.parent_id.is_some() { RichText::new(format!("    {}", c.name)) } else { RichText::new(&c.name).strong() };
            changed |= ui.selectable_value(value, Some(c.id), text).changed();
        }
    });
    changed
}

/// Account picker. `none_label` adds a "no account" entry when non-empty.
pub fn account_combo(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    value: &mut Option<i64>,
    accounts: &[Account],
    none_label: &str,
) -> bool {
    let selected =
        value.and_then(|v| accounts.iter().find(|a| a.id == v)).map_or_else(|| none_label.to_string(), |a| a.name.clone());
    let mut changed = false;
    let current = *value;
    ComboBox::from_id_salt(id).selected_text(selected).show_ui(ui, |ui| {
        if !none_label.is_empty() {
            changed |= ui.selectable_value(value, None, none_label).changed();
        }
        for a in accounts.iter().filter(|a| !a.archived || Some(a.id) == current) {
            changed |= ui.selectable_value(value, Some(a.id), &a.name).changed();
        }
    });
    changed
}

/// Dollar input backed by integer cents.
pub fn money_drag(ui: &mut Ui, value: &mut Money) -> Response {
    let mut dollars = value.dollars();
    let r = ui.add(egui::DragValue::new(&mut dollars).speed(1.0).prefix("$").fixed_decimals(2));
    if r.changed() {
        *value = Money::from_dollars(dollars);
    }
    r
}

/// Date input with a calendar popup.
pub fn date_edit(ui: &mut Ui, id: &str, date: &mut NaiveDate) -> bool {
    ui.add(egui_extras::DatePickerButton::new(date).id_salt(id).calendar_week(false)).changed()
}

/// Amount text: positive amounts in the "money in" text colour.
pub fn money_text(m: Money, palette: &Palette) -> RichText {
    let t = RichText::new(m.to_string()).monospace();
    if m.cents() > 0 { t.color(palette.positive_text) } else { t }
}

/// Headline number tile.
pub fn stat_tile(ui: &mut Ui, title: &str, value: &str, detail: &str, help_text: &str) {
    // Allocating a desired size first lets `horizontal_wrapped` rows wrap tiles.
    ui.allocate_ui_with_layout(egui::vec2(182.0, 76.0), egui::Layout::top_down(egui::Align::Min), |ui| {
        stat_tile_frame(ui, title, value, detail, help_text)
    });
}

fn stat_tile_frame(ui: &mut Ui, title: &str, value: &str, detail: &str, help_text: &str) {
    egui::Frame::group(ui.style()).inner_margin(Margin::same(10)).show(ui, |ui| {
        ui.set_min_width(160.0);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).weak());
                if !help_text.is_empty() {
                    help(ui, help_text);
                }
            });
            ui.label(RichText::new(value).size(22.0).strong());
            if !detail.is_empty() {
                ui.label(RichText::new(detail).small().weak());
            }
        });
    });
}

/// A button that asks for inline confirmation. Returns `true` once confirmed.
pub fn confirm_button(ui: &mut Ui, id: Id, label: &str, prompt: &str) -> bool {
    let armed: bool = ui.data(|d| d.get_temp(id)).unwrap_or(false);
    if !armed {
        if ui.button(label).clicked() {
            ui.data_mut(|d| d.insert_temp(id, true));
        }
        return false;
    }
    let mut confirmed = false;
    ui.label(RichText::new(prompt).strong());
    if ui.button("Yes").clicked() {
        confirmed = true;
        ui.data_mut(|d| d.remove::<bool>(id));
    }
    if ui.button("Cancel").clicked() {
        ui.data_mut(|d| d.remove::<bool>(id));
    }
    confirmed
}

/// Bordered card with a title.
pub fn card<R>(ui: &mut Ui, title: &str, help_text: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::group(ui.style())
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.strong(title);
                if !help_text.is_empty() {
                    help(ui, help_text);
                }
            });
            ui.separator();
            add(ui)
        })
        .inner
}

/// Muted placeholder text for empty states.
pub fn empty_state(ui: &mut Ui, text: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(text).italics().weak());
    ui.add_space(8.0);
}

/// Thin progress bar with its label beside it (text never sits on the coloured fill).
pub fn labeled_bar(ui: &mut Ui, fraction: f64, color: egui::Color32, label: &str, bar_width: f32) {
    ui.horizontal(|ui| {
        ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0) as f32).fill(color).desired_width(bar_width).desired_height(10.0));
        ui.label(label);
    });
}

/// Format a fraction as a percentage.
pub fn pct(f: f64) -> String {
    format!("{:.1}%", f * 100.0)
}
