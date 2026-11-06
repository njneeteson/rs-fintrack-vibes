//! Manage the two-level category hierarchy.

use crate::help::tips;
use crate::model::{Category, CategoryKind, Money};
use crate::ui::state::PageCtx;
use crate::ui::widgets::{category_combo, enum_combo, heading, help, label_with_help};
use egui::{Id, RichText, TextEdit, Ui};
use std::collections::HashMap;

/// Edit dialog state.
struct EditState {
    id: i64,
    name: String,
    parent: Option<i64>,
    kind: CategoryKind,
}

struct DeleteState {
    id: i64,
    reassign_to: Option<i64>,
}

pub struct CategoriesPage {
    new_top: String,
    new_top_kind: CategoryKind,
    new_sub: HashMap<i64, String>,
    edit: Option<EditState>,
    delete: Option<DeleteState>,
}

impl Default for CategoriesPage {
    fn default() -> Self {
        Self { new_top: String::new(), new_top_kind: CategoryKind::Expense, new_sub: HashMap::new(), edit: None, delete: None }
    }
}

/// Transaction count and net total per category id (exact assignment).
fn usage(cx: &PageCtx<'_>) -> HashMap<i64, (usize, Money)> {
    let mut m: HashMap<i64, (usize, Money)> = HashMap::new();
    for t in &cx.data.transactions {
        if let Some(c) = t.category_id() {
            let e = m.entry(c).or_default();
            e.0 += 1;
            e.1 += t.amount;
        }
    }
    m
}

impl CategoriesPage {
    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        heading(ui, "Categories", tips::CATEGORY_KIND);
        ui.horizontal(|ui| {
            ui.label("New top-level category:");
            ui.add(TextEdit::singleline(&mut self.new_top).hint_text("e.g. Pets").desired_width(180.0));
            enum_combo(ui, "new-top-kind", &mut self.new_top_kind);
            help(ui, tips::CATEGORY_KIND);
            if ui.add_enabled(!self.new_top.trim().is_empty(), egui::Button::new("➕ Add")).clicked()
                && cx
                    .out
                    .report(cx.db.add_category(&self.new_top, None, self.new_top_kind), |_| "Category added".into())
                    .is_some()
            {
                self.new_top.clear();
            }
        });
        ui.separator();
        let used = usage(cx);
        let cats = &cx.data.categories;
        let tops: Vec<Category> = cats.top_level().cloned().collect();
        egui::ScrollArea::vertical().id_salt("cats-scroll").show(ui, |ui| {
            for kind in CategoryKind::ALL {
                ui.label(RichText::new(format!("{} categories", kind.label())).size(15.0).strong());
                for top in tops.iter().filter(|c| c.kind == *kind) {
                    self.top_row(ui, cx, top, &used);
                }
                ui.add_space(8.0);
            }
        });
        self.edit_window(ui, cx);
        self.delete_window(ui, cx);
    }

    fn usage_label(used: &HashMap<i64, (usize, Money)>, id: i64) -> String {
        let (n, total) = used.get(&id).copied().unwrap_or_default();
        if n == 0 { "unused".into() } else { format!("{n} txns · {total}") }
    }

    fn actions(&mut self, ui: &mut Ui, c: &Category) {
        if ui.small_button("✏").on_hover_text("Rename, move or change kind").clicked() {
            self.edit = Some(EditState { id: c.id, name: c.name.clone(), parent: c.parent_id, kind: c.kind });
        }
        if ui.small_button("🗑").on_hover_text(tips::CATEGORY_DELETE).clicked() {
            self.delete = Some(DeleteState { id: c.id, reassign_to: None });
        }
    }

    fn top_row(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>, top: &Category, used: &HashMap<i64, (usize, Money)>) {
        let children: Vec<Category> = cx.data.categories.children(top.id).cloned().collect();
        // The header shows the whole branch (category + subcategories).
        let mut rollup: HashMap<i64, (usize, Money)> = HashMap::new();
        let branch = rollup.entry(top.id).or_default();
        for id in std::iter::once(top.id).chain(children.iter().map(|c| c.id)) {
            let (n, total) = used.get(&id).copied().unwrap_or_default();
            branch.0 += n;
            branch.1 += total;
        }
        let header = format!("{}   ({} subcategories · {})", top.name, children.len(), Self::usage_label(&rollup, top.id));
        egui::CollapsingHeader::new(header).id_salt(("cat", top.id)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Category itself:").weak());
                self.actions(ui, top);
            });
            for c in &children {
                ui.horizontal(|ui| {
                    ui.label(format!("  • {}", c.name));
                    ui.label(RichText::new(Self::usage_label(used, c.id)).weak());
                    self.actions(ui, c);
                });
            }
            ui.horizontal(|ui| {
                let text = self.new_sub.entry(top.id).or_default();
                ui.add(TextEdit::singleline(text).hint_text("New subcategory").desired_width(180.0));
                if ui.add_enabled(!text.trim().is_empty(), egui::Button::new("➕ Add subcategory")).clicked() {
                    let name = text.clone();
                    if cx.out.report(cx.db.add_category(&name, Some(top.id), top.kind), |_| format!("Added {}", name)).is_some() {
                        text.clear();
                    }
                }
            });
        });
    }

    fn edit_window(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        let Some(e) = &mut self.edit else { return };
        let cats = &cx.data.categories;
        let has_children = cats.children(e.id).next().is_some();
        let mut close = false;
        egui::Modal::new(Id::new("cat-edit")).show(ui.ctx(), |ui| {
            ui.heading("Edit category");
            egui::Grid::new("cat-edit-grid").num_columns(2).show(ui, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut e.name);
                ui.end_row();
                ui.label("Parent");
                let parent_text = e.parent.map_or("(top-level)".to_string(), |p| cats.full_name(p));
                ui.add_enabled_ui(!has_children, |ui| {
                    egui::ComboBox::from_id_salt("cat-edit-parent").selected_text(parent_text).show_ui(ui, |ui| {
                        ui.selectable_value(&mut e.parent, None, "(top-level)");
                        for c in cats.top_level().filter(|c| c.id != e.id) {
                            ui.selectable_value(&mut e.parent, Some(c.id), &c.name);
                        }
                    });
                })
                .response
                .on_disabled_hover_text("A category with subcategories must stay top-level.");
                ui.end_row();
                if e.parent.is_none() {
                    label_with_help(ui, "Kind", tips::CATEGORY_KIND);
                    enum_combo(ui, "cat-edit-kind", &mut e.kind);
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    close = cx
                        .out
                        .report(cx.db.update_category(e.id, &e.name, e.parent, e.kind), |_| "Category updated".into())
                        .is_some();
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        if close {
            self.edit = None;
        }
    }

    fn delete_window(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        let Some(d) = &mut self.delete else { return };
        let cats = &cx.data.categories;
        let affected: Vec<i64> = std::iter::once(d.id).chain(cats.children(d.id).map(|c| c.id)).collect();
        let txns = cx.data.transactions.iter().filter(|t| t.category_id().is_some_and(|c| affected.contains(&c))).count();
        let rules = cx.data.rules.iter().filter(|r| affected.contains(&r.category_id)).count();
        let mut close = false;
        egui::Modal::new(Id::new("cat-delete")).show(ui.ctx(), |ui| {
            ui.heading(format!("Delete “{}”?", cats.full_name(d.id)));
            if affected.len() > 1 {
                ui.label(format!("Its {} subcategories will be deleted too.", affected.len() - 1));
            }
            ui.label(format!("{txns} transactions and {rules} rules use it."));
            ui.horizontal(|ui| {
                ui.label("Move them to:");
                category_combo(
                    ui,
                    Id::new("cat-reassign"),
                    &mut d.reassign_to,
                    cats,
                    "(don't move — leave uncategorized, delete rules)",
                    260.0,
                );
            });
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Delete").color(cx.palette.critical)).clicked() {
                    close = cx.out.report(cx.db.delete_category(d.id, d.reassign_to), |_| "Category deleted".into()).is_some();
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        if close {
            self.delete = None;
        }
    }
}
