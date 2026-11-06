//! Mapping rules: unmapped transactions, rule list and the rule editor.

mod editor;
mod rules;
mod unmapped;

use crate::help::tips;
use crate::model::Rule;
use crate::ui::state::PageCtx;
use crate::ui::widgets::heading;
use editor::RuleEditor;
use egui::Ui;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Unmapped,
    Rules,
}

#[derive(Default)]
pub struct CategorizePage {
    tab: Tab,
    editor: Option<RuleEditor>,
    unmapped: unmapped::UnmappedView,
    rules: rules::RulesView,
}

impl CategorizePage {
    /// Open the editor with a draft (e.g. from the Transactions page).
    pub fn open_editor(&mut self, rule: Rule) {
        self.editor = Some(RuleEditor::new(rule));
    }

    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        heading(ui, "Mapping rules", tips::RULES);
        let unmapped = cx.data.uncategorized_count();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Unmapped, format!("Unmapped ({unmapped})"));
            ui.selectable_value(&mut self.tab, Tab::Rules, format!("Rules ({})", cx.data.rules.len()));
            ui.separator();
            if ui.button("➕ New rule").clicked() {
                self.open_editor(Rule::default());
            }
            if ui.button("↺ Re-apply all rules").on_hover_text(tips::REAPPLY).clicked() {
                cx.out.report(cx.db.apply_rules(), |n| format!("Rules re-applied — {n} transactions changed"));
            }
        });
        ui.separator();
        let request = match self.tab {
            Tab::Unmapped => self.unmapped.show(ui, cx),
            Tab::Rules => self.rules.show(ui, cx),
        };
        if let Some(rule) = request {
            self.open_editor(rule);
        }
        if let Some(editor) = &mut self.editor
            && !editor.show(ui.ctx(), cx)
        {
            self.editor = None;
        }
    }
}
