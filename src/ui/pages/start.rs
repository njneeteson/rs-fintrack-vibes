//! Start screen shown when no database is open: create, open, recent.

use crate::db::is_database_dir;
use crate::help::tips;
use crate::settings::AppSettings;
use crate::ui::state::{AppAction, Outbox};
use crate::ui::widgets::{card, help};
use egui::{RichText, TextEdit, Ui};
use std::path::PathBuf;

#[derive(Default)]
pub struct StartPage {
    name: String,
    parent: Option<PathBuf>,
}

impl StartPage {
    pub fn show(&mut self, ui: &mut Ui, settings: &AppSettings, out: &mut Outbox) {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.label(RichText::new("FinTrack").size(34.0).strong());
            ui.label(RichText::new("Private, offline household finances").size(16.0).weak());
            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - 360.0).max(0.0) / 2.0);
                ui.label(RichText::new("🔒 No internet connection is ever used.").small());
                help(ui, tips::PRIVACY);
            });
            ui.add_space(16.0);
        });
        let width = ui.available_width().min(640.0);
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - width).max(0.0) / 2.0);
            ui.vertical(|ui| {
                ui.set_width(width);
                card(ui, "New here? Try the demo", tips::DEMO, |ui| {
                    ui.label("Explore FinTrack with a sample household: 18 months of bank and credit-card transactions, rules, budgets and a mortgage.");
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("🎓 Enter demo mode").size(16.0)).clicked() {
                            out.app = Some(AppAction::Demo);
                        }
                        ui.label(RichText::new("Temporary — deleted when you exit. Your own data is never touched.").small().weak());
                    });
                });
                ui.add_space(8.0);
                card(ui, "Create a new database", tips::NEW_DATABASE, |ui| self.create(ui, out));
                ui.add_space(8.0);
                card(ui, "Open an existing database", tips::OPEN_DATABASE, |ui| {
                    if ui.button("📂 Choose database folder…").clicked()
                        && let Some(dir) = rfd::FileDialog::new().set_title("Open FinTrack database folder").pick_folder()
                    {
                        out.app = Some(AppAction::Open(dir));
                    }
                    if settings.recent_databases.is_empty() {
                        return;
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new("Recent").strong());
                    for path in &settings.recent_databases {
                        ui.horizontal(|ui| {
                            let exists = is_database_dir(path);
                            if ui.add_enabled(exists, egui::Button::new("Open")).clicked() {
                                out.app = Some(AppAction::Open(path.clone()));
                            }
                            let mut label = path.display().to_string();
                            if settings.default_database.as_ref() == Some(path) {
                                label.push_str("  (default)");
                            }
                            if !exists {
                                label.push_str("  — not found");
                            }
                            ui.label(label);
                        });
                    }
                });
            });
        });
    }

    fn create(&mut self, ui: &mut Ui, out: &mut Outbox) {
        egui::Grid::new("create-db").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Name");
            ui.add(TextEdit::singleline(&mut self.name).hint_text("e.g. Household Finances").desired_width(260.0));
            ui.end_row();
            ui.label("Location");
            ui.horizontal(|ui| {
                if ui.button("Choose folder…").clicked() {
                    self.parent = rfd::FileDialog::new().set_title("Where should the database folder be created?").pick_folder();
                }
                ui.label(self.parent.as_ref().map_or("not chosen".to_string(), |p| p.display().to_string()));
            });
            ui.end_row();
        });
        let name = self.name.trim();
        let target = self.parent.as_ref().filter(|_| !name.is_empty()).map(|p| p.join(name));
        if let Some(t) = &target {
            ui.label(RichText::new(format!("Will create {}", t.display())).small().weak());
        }
        if ui.add_enabled(target.is_some(), egui::Button::new("✨ Create database")).clicked() {
            out.app = target.map(AppAction::Create);
        }
    }
}
