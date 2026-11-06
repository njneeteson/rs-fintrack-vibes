use super::Controls;
use crate::analytics::DateRange;
use crate::analytics::balances::{balance_on, balance_series};
use crate::analytics::period::periods_covering;
use crate::help::tips;
use crate::model::Money;
use crate::ui::charts::line_chart;
use crate::ui::filters::today;
use crate::ui::state::PageCtx;
use crate::ui::theme::MAX_SERIES;
use crate::ui::widgets::{card, empty_state, money_text, stat_tile};
use egui::Ui;

pub fn show(ui: &mut Ui, cx: &mut PageCtx<'_>, c: &Controls, range: DateRange) {
    let data = cx.data;
    let accounts: Vec<_> = data.accounts.iter().filter(|a| !a.archived).collect();
    if accounts.is_empty() {
        empty_state(ui, "No accounts yet.");
        return;
    }
    let periods = periods_covering(range.start, range.end, c.granularity);
    let per_account: Vec<Vec<Money>> = accounts.iter().map(|a| balance_series(a, &data.transactions, &periods)).collect();
    let net_worth: Vec<f64> = (0..periods.len()).map(|i| per_account.iter().map(|s| s[i]).sum::<Money>().dollars()).collect();
    let today_total: Money = accounts.iter().map(|a| balance_on(a, &data.transactions, today())).sum();
    ui.horizontal_wrapped(|ui| {
        stat_tile(
            ui,
            "Net worth (tracked accounts)",
            &today_total.to_whole(),
            "sum of account balances today",
            tips::OPENING_BALANCE,
        );
    });
    let labels: Vec<String> = periods.iter().map(|p| p.short_label()).collect();
    card(ui, "Net worth", "End-of-period sum of all non-archived account balances.", |ui| {
        line_chart(ui, "net-worth", labels.clone(), &[("Net worth".into(), net_worth, cx.palette.series(0))], 240.0);
    });
    ui.add_space(6.0);
    card(
        ui,
        "Balance by account",
        "Balances are opening balance + all imported transactions. Accounts beyond the palette are listed in the table only.",
        |ui| {
            let series: Vec<(String, Vec<f64>, egui::Color32)> = accounts
                .iter()
                .zip(&per_account)
                .take(MAX_SERIES)
                .enumerate()
                .map(|(i, (a, s))| (a.name.clone(), s.iter().map(|m| m.dollars()).collect(), cx.palette.series(i)))
                .collect();
            line_chart(ui, "account-balances", labels, &series, 280.0);
            egui::Grid::new("balances-table").striped(true).num_columns(2).show(ui, |ui| {
                for a in &accounts {
                    ui.label(&a.name);
                    ui.label(money_text(balance_on(a, &data.transactions, today()), &cx.palette));
                    ui.end_row();
                }
            });
        },
    );
}
