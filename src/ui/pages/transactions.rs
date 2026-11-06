//! Transaction list: filters, per-transaction category overrides, bulk actions, notes, export.

use crate::analytics::RangePreset;
use crate::categorize::suggest_pattern;
use crate::help::tips;
use crate::model::{CategoryTree, Money, Rule, Transaction};
use crate::ui::filters::RangeSelector;
use crate::ui::state::{PageCtx, View};
use crate::ui::widgets::{account_combo, category_combo, category_combo_with_text, confirm_button, heading, help, money_text};
use egui::{Id, RichText, TextEdit, Ui};
use egui_extras::{Column, TableBuilder};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CategoryFilter {
    Any,
    Uncategorized,
    /// A category, optionally including its subcategories.
    Category {
        id: i64,
        include_children: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TxnFilter {
    pub range: RangeSelector,
    pub account: Option<i64>,
    pub category: CategoryFilter,
    pub search: String,
    pub manual_only: bool,
}

impl Default for TxnFilter {
    fn default() -> Self {
        Self {
            range: RangeSelector::new(RangePreset::AllTime),
            account: None,
            category: CategoryFilter::Any,
            search: String::new(),
            manual_only: false,
        }
    }
}

impl TxnFilter {
    fn matches(&self, t: &Transaction, cats: &CategoryTree, range: crate::analytics::DateRange, needle: &str) -> bool {
        range.contains(t.date)
            && self.account.is_none_or(|a| a == t.account_id)
            && (!self.manual_only || t.is_manual())
            && match self.category {
                CategoryFilter::Any => true,
                CategoryFilter::Uncategorized => t.is_uncategorized(),
                CategoryFilter::Category { id, include_children } => {
                    t.category_id().is_some_and(|c| if include_children { cats.is_under(c, id) } else { c == id })
                }
            }
            && (needle.is_empty()
                || t.description.to_lowercase().contains(needle)
                || t.memo.to_lowercase().contains(needle)
                || t.note.to_lowercase().contains(needle))
    }
}

#[derive(Default)]
pub struct TransactionsPage {
    pub filter: TxnFilter,
    /// `(data generation, filter, matching indexes)`
    cache: Option<(u64, TxnFilter, Vec<usize>)>,
    selected: HashSet<i64>,
    bulk_category: Option<i64>,
    note_editor: Option<(i64, String)>,
}

impl TransactionsPage {
    pub fn set_filter(&mut self, filter: TxnFilter) {
        self.filter = filter;
        self.selected.clear();
    }

    fn visible(&mut self, cx: &PageCtx<'_>) -> Vec<usize> {
        let gen_ = cx.data.generation;
        if let Some((g, f, idx)) = &self.cache
            && *g == gen_
            && *f == self.filter
        {
            return idx.clone();
        }
        let range = self.filter.range.resolve(cx.data);
        let needle = self.filter.search.trim().to_lowercase();
        let idx: Vec<usize> = cx
            .data
            .transactions
            .iter()
            .enumerate()
            .filter(|(_, t)| self.filter.matches(t, &cx.data.categories, range, &needle))
            .map(|(i, _)| i)
            .collect();
        self.cache = Some((gen_, self.filter.clone(), idx.clone()));
        idx
    }

    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        heading(ui, "Transactions", tips::OVERRIDE);
        self.filter_bar(ui, cx);
        let visible = self.visible(cx);
        let txns = &cx.data.transactions;
        self.selected.retain(|id| visible.iter().any(|&i| txns[i].id == *id));

        let (inflow, outflow): (Money, Money) = visible
            .iter()
            .map(|&i| txns[i].amount)
            .fold((Money::ZERO, Money::ZERO), |(a, b), m| if m.cents() > 0 { (a + m, b) } else { (a, b + m) });
        ui.horizontal(|ui| {
            ui.label(format!("{} transactions", visible.len()));
            ui.separator();
            ui.label("In:");
            ui.label(money_text(inflow, &cx.palette));
            ui.label("Out:");
            ui.label(money_text(outflow, &cx.palette));
            ui.label("Net:");
            ui.label(money_text(inflow + outflow, &cx.palette));
            ui.separator();
            if ui.button("Export CSV…").on_hover_text("Save the filtered transactions as a CSV file.").clicked() {
                export_csv(cx, &visible);
            }
        });
        self.bulk_bar(ui, cx);
        ui.separator();
        self.table(ui, cx, &visible);
        self.note_window(ui, cx);
    }

    fn filter_bar(&mut self, ui: &mut Ui, cx: &PageCtx<'_>) {
        let data = cx.data;
        ui.horizontal_wrapped(|ui| {
            ui.label("Dates:");
            self.filter.range.ui(ui, "txn-range", data);
            ui.separator();
            ui.label("Account:");
            account_combo(ui, "txn-account", &mut self.filter.account, &data.accounts, "All accounts");
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Category:");
            let mut mode = match self.filter.category {
                CategoryFilter::Any => 0,
                CategoryFilter::Uncategorized => 1,
                CategoryFilter::Category { .. } => 2,
            };
            egui::ComboBox::from_id_salt("txn-cat-mode").selected_text(["Any", "Uncategorized", "Specific…"][mode]).show_ui(
                ui,
                |ui| {
                    ui.selectable_value(&mut mode, 0, "Any");
                    ui.selectable_value(&mut mode, 1, "Uncategorized");
                    ui.selectable_value(&mut mode, 2, "Specific…");
                },
            );
            self.filter.category = match (mode, self.filter.category) {
                (0, _) => CategoryFilter::Any,
                (1, _) => CategoryFilter::Uncategorized,
                (_, c @ CategoryFilter::Category { .. }) => c,
                _ => CategoryFilter::Category {
                    id: data.categories.top_level().next().map_or(0, |c| c.id),
                    include_children: true,
                },
            };
            if let CategoryFilter::Category { id, include_children } = &mut self.filter.category {
                let mut sel = Some(*id);
                if category_combo(ui, Id::new("txn-cat"), &mut sel, &data.categories, "", 200.0) {
                    *id = sel.unwrap_or(*id);
                }
                ui.checkbox(include_children, "incl. subcategories");
            }
            ui.separator();
            ui.add(
                TextEdit::singleline(&mut self.filter.search).hint_text("Search description, memo, note").desired_width(220.0),
            );
            ui.checkbox(&mut self.filter.manual_only, "Manual overrides only");
            if ui.button("Reset").clicked() {
                self.filter = TxnFilter::default();
            }
        });
    }

    fn bulk_bar(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        if self.selected.is_empty() {
            return;
        }
        let ids: Vec<i64> = self.selected.iter().copied().collect();
        ui.horizontal(|ui| {
            ui.strong(format!("{} selected:", ids.len()));
            category_combo(ui, Id::new("bulk-cat"), &mut self.bulk_category, &cx.data.categories, "Choose category…", 200.0);
            if ui.add_enabled(self.bulk_category.is_some(), egui::Button::new("Set category")).clicked() {
                let n = ids.len();
                cx.out
                    .report(cx.db.set_manual_category(&ids, self.bulk_category), |_| format!("Set category on {n} transactions"));
            }
            if ui.button("Clear overrides").on_hover_text("Return to the automatic (rule) category.").clicked() {
                cx.out.report(cx.db.set_manual_category(&ids, None), |_| "Overrides cleared".into());
            }
            if confirm_button(ui, Id::new("bulk-delete"), "🗑 Delete", "Delete the selected transactions?") {
                cx.out.report(cx.db.delete_transactions(&ids), |_| "Transactions deleted".into());
                self.selected.clear();
            }
            if ui.button("Select none").clicked() {
                self.selected.clear();
            }
        });
    }

    fn table(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>, visible: &[usize]) {
        let data = cx.data;
        let cats = &data.categories;
        let mut all_selected = !visible.is_empty() && self.selected.len() == visible.len();
        TableBuilder::new(ui)
            .id_salt("txn-table")
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(24.0))
            .column(Column::exact(86.0))
            .column(Column::initial(130.0).clip(true))
            .column(Column::remainder().at_least(200.0).clip(true))
            .column(Column::exact(110.0))
            .column(Column::initial(230.0))
            .column(Column::exact(60.0))
            .header(22.0, |mut h| {
                h.col(|ui| {
                    if ui.checkbox(&mut all_selected, "").on_hover_text("Select all shown").changed() {
                        self.selected = if all_selected {
                            visible.iter().map(|&i| data.transactions[i].id).collect()
                        } else {
                            HashSet::new()
                        };
                    }
                });
                for title in ["Date", "Account", "Description", "Amount"] {
                    h.col(|ui| {
                        ui.strong(title);
                    });
                }
                h.col(|ui| {
                    ui.strong("Category");
                    help(ui, tips::OVERRIDE);
                });
                h.col(|ui| {
                    ui.strong("");
                });
            })
            .body(|body| {
                body.rows(24.0, visible.len(), |mut row| {
                    let t = &data.transactions[visible[row.index()]];
                    row.col(|ui| {
                        let mut sel = self.selected.contains(&t.id);
                        if ui.checkbox(&mut sel, "").changed() {
                            if sel {
                                self.selected.insert(t.id)
                            } else {
                                self.selected.remove(&t.id)
                            };
                        }
                    });
                    row.col(|ui| {
                        ui.label(t.date.format("%Y-%m-%d").to_string());
                    });
                    row.col(|ui| {
                        ui.label(data.account_name(t.account_id));
                    });
                    row.col(|ui| {
                        let mut text = RichText::new(&t.description);
                        if !t.note.is_empty() {
                            text = RichText::new(format!("{}  📝 {}", t.description, t.note));
                        }
                        let mut hover = t.description.clone();
                        if !t.memo.is_empty() {
                            hover.push_str(&format!("\nMemo: {}", t.memo));
                        }
                        if !t.note.is_empty() {
                            hover.push_str(&format!("\nNote: {}", t.note));
                        }
                        ui.label(text).on_hover_text(hover);
                    });
                    row.col(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(money_text(t.amount, &cx.palette));
                        });
                    });
                    row.col(|ui| {
                        if let Some(choice) = override_combo(ui, t, cats, data) {
                            cx.out.report(cx.db.set_manual_category(&[t.id], choice), |_| String::new());
                        }
                    });
                    row.col(|ui| {
                        if ui.small_button("✏").on_hover_text("Add or edit a note").clicked() {
                            self.note_editor = Some((t.id, t.note.clone()));
                        }
                        if ui.small_button("➕").on_hover_text("Create a mapping rule from this transaction").clicked() {
                            cx.out.rule_draft = Some(Rule {
                                pattern: suggest_pattern(&t.description),
                                category_id: t.category_id().unwrap_or(0),
                                ..Default::default()
                            });
                            cx.out.navigate = Some(View::Categorize);
                        }
                    });
                });
            });
    }

    fn note_window(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        let Some((id, text)) = &mut self.note_editor else { return };
        let mut close = false;
        egui::Window::new("Transaction note").collapsible(false).resizable(false).show(ui.ctx(), |ui| {
            ui.add(TextEdit::multiline(text).desired_rows(3).desired_width(320.0));
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    cx.out.report(cx.db.set_note(*id, text.trim()), |_| "Note saved".into());
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        if close {
            self.note_editor = None;
        }
    }
}

/// Category cell: shows the effective category; choosing one sets a manual
/// override, "Automatic" clears it. Returns the new override when changed.
fn override_combo(ui: &mut Ui, t: &Transaction, cats: &CategoryTree, data: &crate::ui::data::DataCache) -> Option<Option<i64>> {
    let text = match (t.manual_category_id, t.auto_category_id) {
        (Some(m), _) => RichText::new(format!("✋ {}", cats.full_name(m))),
        (None, Some(a)) => RichText::new(cats.full_name(a)),
        (None, None) => RichText::new("Uncategorized").italics().weak(),
    };
    let auto_label = match t.auto_category_id {
        Some(a) => format!("Automatic ({})", cats.full_name(a)),
        None => "Automatic (no rule matches)".to_string(),
    };
    let mut value = t.manual_category_id;
    let changed = category_combo_with_text(ui, Id::new(("txn-override", t.id)), &mut value, cats, &auto_label, 210.0, text);
    let resp_hover = match (t.manual_category_id, t.rule_id.and_then(|r| data.rules.iter().find(|x| x.id == r))) {
        (Some(_), _) => "Manual override".to_string(),
        (None, Some(rule)) => format!("From rule: {}", rule.describe()),
        _ => String::new(),
    };
    if !resp_hover.is_empty() {
        ui.label(RichText::new("ℹ").weak()).on_hover_text(resp_hover);
    }
    changed.then_some(value)
}

fn export_csv(cx: &mut PageCtx<'_>, visible: &[usize]) {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Export transactions")
        .add_filter("CSV", &["csv"])
        .set_file_name("transactions.csv")
        .save_file()
    else {
        return;
    };
    let result = (|| -> anyhow::Result<usize> {
        let mut w = csv::Writer::from_path(&path)?;
        w.write_record(["Date", "Account", "Description", "Memo", "Amount", "Category", "Manual", "Note"])?;
        for &i in visible {
            let t = &cx.data.transactions[i];
            w.write_record([
                t.date.to_string(),
                cx.data.account_name(t.account_id),
                t.description.clone(),
                t.memo.clone(),
                t.amount.to_plain(),
                t.category_id().map(|c| cx.data.categories.full_name(c)).unwrap_or_default(),
                if t.is_manual() { "yes".into() } else { String::new() },
                t.note.clone(),
            ])?;
        }
        w.flush()?;
        Ok(visible.len())
    })();
    match result {
        Ok(n) => cx.out.info(format!("Exported {n} transactions to {}", path.display())),
        Err(e) => cx.out.error(format!("Export failed: {e:#}")),
    }
}
