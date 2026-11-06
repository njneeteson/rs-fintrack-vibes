//! Analytics: shared controls plus one module per tab.

mod balances;
mod breakdown;
mod cash_flow;
mod payees;
mod recurring;
mod trends;

use crate::analytics::{BucketKey, Bucketing, DateRange, Flow, Granularity, RangePreset};
use crate::help::tips;
use crate::ui::filters::RangeSelector;
use crate::ui::pages::transactions::{CategoryFilter, TxnFilter};
use crate::ui::state::{PageCtx, View};
use crate::ui::theme::MAX_SERIES;
use crate::ui::widgets::{enum_combo, heading, help};
use egui::Ui;

str_enum! {
    pub enum Tab {
        CashFlow => ("cash_flow", "Cash flow"),
        Breakdown => ("breakdown", "Breakdown"),
        Trends => ("trends", "Trends"),
        Payees => ("payees", "Payees"),
        Recurring => ("recurring", "Recurring"),
        Balances => ("balances", "Balances & net worth"),
    }
}

/// Controls shared by all tabs.
#[derive(Clone, Debug, PartialEq)]
pub struct Controls {
    pub range: RangeSelector,
    pub granularity: Granularity,
    pub bucketing: Bucketing,
    pub flow: Flow,
    pub top_n: usize,
}

pub struct AnalyticsPage {
    tab: Tab,
    controls: Controls,
    trends: trends::TrendsState,
}

impl Default for AnalyticsPage {
    fn default() -> Self {
        Self {
            tab: Tab::CashFlow,
            controls: Controls {
                range: RangeSelector::new(RangePreset::Last12Months),
                granularity: Granularity::Month,
                bucketing: Bucketing::TopCategory,
                flow: Flow::Expense,
                top_n: 7,
            },
            trends: Default::default(),
        }
    }
}

impl AnalyticsPage {
    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        heading(ui, "Analytics", "Where money comes from and where it goes. Transfers are excluded.");
        ui.horizontal(|ui| {
            for t in Tab::ALL {
                ui.selectable_value(&mut self.tab, *t, t.label());
            }
        });
        ui.separator();
        self.controls_bar(ui, cx);
        ui.separator();
        let range = self.controls.range.resolve(cx.data);
        egui::ScrollArea::vertical().id_salt("analytics-scroll").show(ui, |ui| match self.tab {
            Tab::CashFlow => cash_flow::show(ui, cx, &self.controls, range),
            Tab::Breakdown => breakdown::show(ui, cx, &self.controls, range),
            Tab::Trends => self.trends.show(ui, cx, &self.controls, range),
            Tab::Payees => payees::show(ui, cx, &self.controls, range),
            Tab::Recurring => recurring::show(ui, cx),
            Tab::Balances => balances::show(ui, cx, &self.controls, range),
        });
    }

    fn controls_bar(&mut self, ui: &mut Ui, cx: &PageCtx<'_>) {
        let c = &mut self.controls;
        let uses_flow = matches!(self.tab, Tab::Breakdown | Tab::Trends | Tab::Payees);
        let uses_bucket = matches!(self.tab, Tab::Breakdown | Tab::Trends);
        ui.horizontal_wrapped(|ui| {
            if self.tab != Tab::Recurring {
                ui.label("Range:");
                c.range.ui(ui, "analytics-range", cx.data);
                ui.separator();
            }
            if !matches!(self.tab, Tab::Payees | Tab::Recurring) {
                ui.label("Period:");
                enum_combo(ui, "an-gran", &mut c.granularity);
                help(ui, tips::GRANULARITY);
            }
            if uses_flow {
                ui.separator();
                ui.label("Show:");
                enum_combo(ui, "an-flow", &mut c.flow);
                help(ui, tips::FLOW);
            }
            if uses_bucket {
                ui.separator();
                ui.label("Group by:");
                enum_combo(ui, "an-bucket", &mut c.bucketing);
                help(ui, tips::BUCKETING);
            }
            if self.tab == Tab::Breakdown {
                ui.separator();
                ui.label("Series:");
                ui.add(egui::DragValue::new(&mut c.top_n).range(1..=MAX_SERIES - 1));
                help(ui, tips::TOP_N);
            }
        });
    }
}

/// Jump to the Transactions page showing the transactions behind a bucket.
pub fn drill_down(cx: &mut PageCtx<'_>, key: &BucketKey, bucketing: Bucketing, range: DateRange) {
    let mut f = TxnFilter { range: RangeSelector { preset: RangePreset::Custom, custom: range }, ..Default::default() };
    match key {
        BucketKey::Category(id) => {
            f.category = CategoryFilter::Category { id: *id, include_children: bucketing == Bucketing::TopCategory }
        }
        BucketKey::Uncategorized => f.category = CategoryFilter::Uncategorized,
        BucketKey::Account(id) => f.account = Some(*id),
        BucketKey::Payee(p) => f.search = p.to_lowercase(),
        BucketKey::Other => return,
    }
    cx.out.txn_filter = Some(f);
    cx.out.navigate = Some(View::Transactions);
}
