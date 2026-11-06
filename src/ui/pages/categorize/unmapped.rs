//! Uncategorized transactions grouped by payee, with one-click rule creation.

use crate::categorize::suggest_pattern;
use crate::help::tips;
use crate::model::{Money, Rule, Transaction};
use crate::ui::pages::transactions::{CategoryFilter, TxnFilter};
use crate::ui::state::{PageCtx, View};
use crate::ui::widgets::{category_combo, empty_state, help, money_text};
use chrono::NaiveDate;
use egui::{Id, RichText, TextEdit, Ui};
use std::collections::{BTreeSet, HashMap};

struct Group {
    key: String,
    ids: Vec<i64>,
    total: Money,
    first: NaiveDate,
    last: NaiveDate,
    samples: BTreeSet<String>,
}

#[derive(Default)]
pub struct UnmappedView {
    search: String,
    cache: Option<(u64, Vec<Group>)>,
    assign: HashMap<String, Option<i64>>,
}

fn build_groups(txns: &[Transaction]) -> Vec<Group> {
    let mut map: HashMap<String, Group> = HashMap::new();
    for t in txns.iter().filter(|t| t.is_uncategorized()) {
        let key = suggest_pattern(&t.description);
        let g = map.entry(key.clone()).or_insert_with(|| Group {
            key,
            ids: Vec::new(),
            total: Money::ZERO,
            first: t.date,
            last: t.date,
            samples: BTreeSet::new(),
        });
        g.ids.push(t.id);
        g.total += t.amount;
        g.first = g.first.min(t.date);
        g.last = g.last.max(t.date);
        if g.samples.len() < 4 {
            g.samples.insert(t.description.clone());
        }
    }
    let mut groups: Vec<Group> = map.into_values().collect();
    groups.sort_by(|a, b| b.ids.len().cmp(&a.ids.len()).then_with(|| b.total.abs().cmp(&a.total.abs())));
    groups
}

impl UnmappedView {
    /// Returns a rule draft when the user asks to create a rule.
    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) -> Option<Rule> {
        let generation = cx.data.generation;
        if self.cache.as_ref().is_none_or(|(g, _)| *g != generation) {
            self.cache = Some((generation, build_groups(&cx.data.transactions)));
        }
        let groups = &self.cache.as_ref().expect("just built").1;
        let mut request = None;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Uncategorized transactions, grouped by payee.").weak());
            help(ui, tips::UNMAPPED);
            ui.add(TextEdit::singleline(&mut self.search).hint_text("Filter…").desired_width(180.0));
            if ui
                .button("Open as a list in Transactions")
                .on_hover_text("Categorize individual transactions one by one")
                .clicked()
            {
                cx.out.txn_filter = Some(TxnFilter { category: CategoryFilter::Uncategorized, ..Default::default() });
                cx.out.navigate = Some(View::Transactions);
            }
        });
        if groups.is_empty() {
            empty_state(ui, "🎉 Every transaction has a category.");
            return None;
        }
        let needle = self.search.to_lowercase();
        egui::ScrollArea::vertical().id_salt("unmapped-scroll").show(ui, |ui| {
            egui::Grid::new("unmapped-grid").striped(true).num_columns(5).spacing([12.0, 6.0]).show(ui, |ui| {
                for h in ["Payee", "Count", "Total", "Dates", "Categorize"] {
                    ui.strong(h);
                }
                ui.end_row();
                for g in groups.iter().filter(|g| needle.is_empty() || g.key.to_lowercase().contains(&needle)) {
                    let samples = g.samples.iter().cloned().collect::<Vec<_>>().join("\n");
                    ui.label(RichText::new(&g.key).strong()).on_hover_text(format!("Examples:\n{samples}"));
                    ui.label(g.ids.len().to_string());
                    ui.label(money_text(g.total, &cx.palette));
                    ui.label(RichText::new(format!("{} – {}", g.first, g.last)).weak());
                    ui.horizontal(|ui| {
                        if ui
                            .button("Create rule…")
                            .on_hover_text("Map every transaction like this, now and in future imports")
                            .clicked()
                        {
                            request = Some(Rule { pattern: g.key.clone(), ..Default::default() });
                        }
                        let choice = self.assign.entry(g.key.clone()).or_default();
                        category_combo(
                            ui,
                            Id::new(("unmapped-assign", &g.key)),
                            choice,
                            &cx.data.categories,
                            "Pick category…",
                            180.0,
                        );
                        if ui
                            .add_enabled(choice.is_some(), egui::Button::new("Assign once"))
                            .on_hover_text("Set a manual category on these transactions only (no rule)")
                            .clicked()
                        {
                            let n = g.ids.len();
                            cx.out
                                .report(cx.db.set_manual_category(&g.ids, *choice), |_| format!("Categorized {n} transactions"));
                        }
                    });
                    ui.end_row();
                }
            });
        });
        request
    }
}
