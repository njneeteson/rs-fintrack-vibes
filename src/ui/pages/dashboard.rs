//! At-a-glance overview of the current month.

use super::budgets::budget_bar;
use crate::analytics::balances::balance_on;
use crate::analytics::budget::budget_report;
use crate::analytics::recurring::detect_recurring;
use crate::analytics::summary::{Summary, summarize};
use crate::analytics::{Bucketing, DateRange, Flow, Granularity, Period, breakdown, income_vs_expense};
use crate::help::tips;
use crate::model::Money;
use crate::ui::charts::{bar_list, cash_flow_chart};
use crate::ui::filters::today;
use crate::ui::pages::analytics::drill_down;
use crate::ui::state::{PageCtx, View};
use crate::ui::widgets::{card, empty_state, heading, money_text, pct, stat_tile};
use chrono::{Days, Months};
use egui::{RichText, Ui};

#[derive(Default)]
pub struct DashboardPage;

fn change(now: Money, before: Money) -> String {
    let d = now - before;
    let sign = if d.cents() >= 0 { "+" } else { "" };
    format!("{sign}{} vs last month", d.to_whole())
}

impl DashboardPage {
    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        heading(ui, &format!("Dashboard — {}", cx.db.name()), "");
        if cx.data.transactions.is_empty() {
            onboarding(ui, cx);
            return;
        }
        let data = cx.data;
        let today = today();
        let month = Period::containing(today, Granularity::Month);
        let this = DateRange::new(month.start, today);
        let last = DateRange::new(month.prev().start, month.prev().last_day());
        let s: Summary = summarize(&data.transactions, this, &data.categories);
        let p: Summary = summarize(&data.transactions, last, &data.categories);

        egui::ScrollArea::vertical().id_salt("dash-scroll").show(ui, |ui| {
            ui.label(RichText::new(format!("This month ({})", month.label())).strong());
            ui.horizontal_wrapped(|ui| {
                stat_tile(ui, "Income", &s.income.to_whole(), &change(s.income, p.income), "");
                stat_tile(ui, "Spending", &s.expense.to_whole(), &change(s.expense, p.expense), tips::FLOW);
                stat_tile(ui, "Net", &s.net().to_whole(), &change(s.net(), p.net()), "");
                stat_tile(
                    ui,
                    "Savings rate",
                    &s.savings_rate().map_or("—".into(), pct),
                    &format!("last month {}", p.savings_rate().map_or("—".into(), pct)),
                    tips::SAVINGS_RATE,
                );
            });
            let unmapped = data.uncategorized_count();
            if unmapped > 0 {
                ui.horizontal(|ui| {
                    ui.colored_label(cx.palette.warning, format!("⚠ {unmapped} transactions are uncategorized."));
                    if ui.button("Categorize now").clicked() {
                        cx.out.navigate = Some(View::Categorize);
                    }
                });
            }
            ui.add_space(6.0);
            ui.columns(2, |cols| {
                card(&mut cols[0], "Spending this month", "Top-level categories. Click one to see its transactions.", |ui| {
                    let b = breakdown(
                        &data.transactions,
                        this,
                        Granularity::Month,
                        Flow::Expense,
                        Bucketing::TopCategory,
                        data.lookup(),
                    );
                    if b.buckets.is_empty() {
                        empty_state(ui, "No spending yet this month.");
                    }
                    let items: Vec<(String, Money)> = b.buckets.iter().take(8).map(|x| (x.label.clone(), x.total)).collect();
                    if let Some(i) = bar_list(ui, &items, cx.palette.spending, b.grand_total()) {
                        drill_down(cx, &b.buckets[i].key, Bucketing::TopCategory, this);
                    }
                });
                card(&mut cols[1], "Budgets this month", tips::BUDGET, |ui| {
                    let mut lines = budget_report(&data.budgets, &data.transactions, &data.categories, month);
                    if lines.is_empty() {
                        empty_state(ui, "No budgets set.");
                    }
                    lines.sort_by(|a, b| b.fraction().total_cmp(&a.fraction()));
                    for l in lines.iter().take(8) {
                        ui.horizontal(|ui| {
                            ui.add_sized([150.0, 18.0], egui::Label::new(&l.label).truncate());
                            budget_bar(ui, l, &cx.palette, (ui.available_width() - 130.0).max(80.0));
                            ui.label(RichText::new(format!("{} / {}", l.actual.to_whole(), l.budget.to_whole())).small());
                        });
                    }
                    if ui.small_button("Manage budgets").clicked() {
                        cx.out.navigate = Some(View::Budgets);
                    }
                });
            });
            ui.add_space(6.0);
            card(ui, "Last 12 months", "Income and spending per month; the line is net.", |ui| {
                let range = DateRange::new(month.start - Months::new(11), today);
                let totals = income_vs_expense(&data.transactions, range, Granularity::Month, &data.categories);
                cash_flow_chart(ui, "dash-cashflow", &totals, &cx.palette, 240.0);
            });
            ui.add_space(6.0);
            ui.columns(2, |cols| {
                card(&mut cols[0], "Accounts", tips::OPENING_BALANCE, |ui| {
                    egui::Grid::new("dash-accounts").num_columns(2).striped(true).show(ui, |ui| {
                        let mut total = Money::ZERO;
                        for a in data.accounts.iter().filter(|a| !a.archived) {
                            let b = balance_on(a, &data.transactions, today);
                            total += b;
                            ui.label(&a.name);
                            ui.label(money_text(b, &cx.palette));
                            ui.end_row();
                        }
                        ui.strong("Total");
                        ui.label(money_text(total, &cx.palette).strong());
                        ui.end_row();
                    });
                });
                card(&mut cols[1], "Upcoming recurring payments (next 30 days)", tips::RECURRING, |ui| {
                    let horizon = today + Days::new(30);
                    let upcoming: Vec<_> = detect_recurring(&data.transactions, &data.categories, today)
                        .into_iter()
                        .filter(|r| r.next_expected <= horizon)
                        .collect();
                    if upcoming.is_empty() {
                        empty_state(ui, "Nothing detected.");
                    }
                    egui::Grid::new("dash-recurring").num_columns(3).striped(true).show(ui, |ui| {
                        for r in upcoming.iter().take(8) {
                            ui.label(&r.payee);
                            ui.label(r.next_expected.format("%b %d").to_string());
                            ui.label(RichText::new(r.typical_amount.to_string()).monospace());
                            ui.end_row();
                        }
                    });
                });
            });
        });
    }
}

fn onboarding(ui: &mut Ui, cx: &mut PageCtx<'_>) {
    card(ui, "Welcome! Let's get your finances in.", tips::PRIVACY, |ui| {
        ui.label("1. Import a CSV export from your bank. The format and account are detected automatically, and you can create the account during import.");
        if ui.button("📥 Import CSV").clicked() {
            cx.out.navigate = Some(View::Import);
        }
        ui.add_space(4.0);
        ui.label(
            "2. Categorize: create mapping rules for unmapped transactions so future imports are categorized automatically.",
        );
        ui.label("3. Explore the Dashboard, Analytics and Budgets. Track your mortgage on the Mortgage page.");
        ui.add_space(4.0);
        if ui.button("❓ Read the guide").clicked() {
            cx.out.navigate = Some(View::Help);
        }
    });
}
