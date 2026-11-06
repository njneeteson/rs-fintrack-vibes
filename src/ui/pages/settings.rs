//! Application settings page (works with or without an open database).

use crate::help::tips;
use crate::settings::AppSettings;
use crate::ui::state::Outbox;
use crate::ui::widgets::{card, enum_combo, heading};
use egui::{RichText, Ui};
use std::path::Path;

pub fn show(ui: &mut Ui, settings: &mut AppSettings, current_db: Option<&Path>, out: &mut Outbox) {
    heading(ui, "Settings", "");
    let before = settings.clone();
    card(ui, "Startup", tips::DEFAULT_DATABASE, |ui| {
        ui.horizontal(|ui| {
            ui.label("Default database:");
            ui.label(
                RichText::new(settings.default_database.as_ref().map_or("none".into(), |p| p.display().to_string())).strong(),
            );
        });
        ui.horizontal(|ui| {
            if let Some(cur) = current_db
                && ui.button("Use the current database").clicked()
            {
                settings.default_database = Some(cur.to_path_buf());
            }
            if ui.button("Choose folder…").clicked()
                && let Some(dir) = rfd::FileDialog::new().set_title("Default FinTrack database").pick_folder()
            {
                if crate::db::is_database_dir(&dir) {
                    settings.default_database = Some(crate::db::resolve_dir(&dir));
                } else {
                    out.error(format!("{} is not a FinTrack database folder", dir.display()));
                }
            }
            if settings.default_database.is_some() && ui.button("Clear").clicked() {
                settings.default_database = None;
            }
        });
        ui.checkbox(&mut settings.open_default_on_startup, "Open the default database when FinTrack starts");
    });
    ui.add_space(6.0);
    card(ui, "Appearance", "", |ui| {
        ui.horizontal(|ui| {
            ui.label("Theme:");
            enum_combo(ui, "theme", &mut settings.theme);
        });
    });
    ui.add_space(6.0);
    card(ui, "Importing", "", |ui| {
        ui.checkbox(&mut settings.archive_imports, "Keep a copy of every imported CSV in the database's imports/ folder");
        ui.checkbox(&mut settings.auto_categorize_on_import, "Apply mapping rules to newly imported transactions");
    });
    ui.add_space(6.0);
    card(ui, "Privacy & files", tips::PRIVACY, |ui| {
        ui.label("FinTrack contains no networking code and never connects to the internet.");
        ui.label(format!("Settings file: {}", AppSettings::default_path().display()));
        if !settings.recent_databases.is_empty() && ui.button("Clear recent databases list").clicked() {
            settings.recent_databases.clear();
        }
    });
    if *settings != before {
        out.settings_changed = true;
    }
}
