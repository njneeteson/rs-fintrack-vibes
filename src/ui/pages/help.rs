//! In-app user guide built from [`crate::help::TOPICS`].

use crate::help::TOPICS;
use crate::ui::widgets::heading;
use egui::{RichText, TextEdit, Ui};

#[derive(Default)]
pub struct HelpPage {
    search: String,
}

impl HelpPage {
    pub fn show(&mut self, ui: &mut Ui) {
        heading(ui, "Help & user guide", "");
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.search).hint_text("Search the guide…").desired_width(260.0));
            ui.label(RichText::new("Tip: hover any ℹ marker in the app for context-specific help.").weak());
        });
        ui.separator();
        let needle = self.search.to_lowercase();
        egui::ScrollArea::vertical().id_salt("help-scroll").show(ui, |ui| {
            for (i, topic) in TOPICS.iter().enumerate() {
                let hit = needle.is_empty()
                    || topic.title.to_lowercase().contains(&needle)
                    || topic.body.to_lowercase().contains(&needle);
                if !hit {
                    continue;
                }
                egui::CollapsingHeader::new(RichText::new(topic.title).size(16.0).strong())
                    .id_salt(("help", i))
                    .default_open(i == 0 || !needle.is_empty())
                    .show(ui, |ui| {
                        for para in topic.body.split('\n') {
                            ui.label(para);
                            ui.add_space(4.0);
                        }
                    });
            }
        });
    }
}
