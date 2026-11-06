//! Mortgage tracker: progress to date and projections with prepayments and rate changes.

mod form;
mod projection;

use crate::model::{Money, Mortgage};
use crate::ui::state::PageCtx;
use crate::ui::widgets::{confirm_button, empty_state, heading};
use egui::{Id, Ui};
use egui_extras::{Size, StripBuilder};

#[derive(Default)]
pub struct MortgagePage {
    selected: Option<i64>,
    /// Unsaved edits of the selected mortgage; projections use it live.
    draft: Option<Mortgage>,
    /// What-if extra amount per payment (not saved).
    extra: Money,
    form: form::FormState,
    full_schedule: bool,
}

impl MortgagePage {
    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        heading(ui, "Mortgage", crate::help::tips::MORTGAGE_RATE);
        let mortgages = &cx.data.mortgages;
        if self.selected.is_none_or(|id| !mortgages.iter().any(|m| m.id == id)) {
            self.selected = mortgages.first().map(|m| m.id);
            self.draft = None;
        }
        ui.horizontal(|ui| {
            if !mortgages.is_empty() {
                let current = self.selected.and_then(|id| mortgages.iter().find(|m| m.id == id));
                egui::ComboBox::from_id_salt("mortgage-select").selected_text(current.map_or("", |m| m.name.as_str())).show_ui(
                    ui,
                    |ui| {
                        for m in mortgages {
                            if ui.selectable_value(&mut self.selected, Some(m.id), &m.name).clicked() {
                                self.draft = None;
                            }
                        }
                    },
                );
            }
            if ui.button("➕ Add mortgage").clicked()
                && let Some(id) =
                    cx.out.report(cx.db.save_mortgage(&Mortgage::default()), |_| "Mortgage added — fill in its details".into())
            {
                self.selected = Some(id);
                self.draft = None;
            }
            if let Some(id) = self.selected
                && confirm_button(ui, Id::new(("del-mortgage", id)), "🗑 Delete", "Delete this mortgage and its prepayments?")
            {
                cx.out.report(cx.db.delete_mortgage(id), |_| "Mortgage deleted".into());
                self.selected = None;
                self.draft = None;
            }
        });
        let Some(saved) = self.selected.and_then(|id| mortgages.iter().find(|m| m.id == id)).cloned() else {
            empty_state(ui, "Add a mortgage to track its progress and see how prepayments change the payoff date and interest.");
            return;
        };
        if self.draft.as_ref().is_none_or(|d| d.id != saved.id) {
            self.draft = Some(saved.clone());
        }
        ui.separator();
        StripBuilder::new(ui).size(Size::exact(400.0)).size(Size::remainder().at_least(400.0)).horizontal(|mut strip| {
            strip.cell(|ui| {
                egui::ScrollArea::vertical().id_salt("mortgage-inputs-scroll").show(ui, |ui| {
                    let draft = self.draft.as_mut().expect("set above");
                    self.form.show(ui, cx, draft, &saved, &mut self.extra);
                });
            });
            strip.cell(|ui| {
                egui::ScrollArea::vertical().id_salt("mortgage-results-scroll").show(ui, |ui| {
                    let draft = self.draft.as_ref().expect("set above");
                    projection::show(ui, cx, draft, self.extra, &mut self.full_schedule);
                });
            });
        });
    }
}
