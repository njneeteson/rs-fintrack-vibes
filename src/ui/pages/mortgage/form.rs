//! Mortgage inputs: details, rate changes, prepayments and the what-if extra payment.

use crate::help::tips;
use crate::model::{Money, Mortgage, Prepayment, RateChange};
use crate::mortgage::regular_payment;
use crate::ui::filters::today;
use crate::ui::state::PageCtx;
use crate::ui::widgets::{card, date_edit, enum_combo, help, label_with_help, money_drag};
use chrono::NaiveDate;
use egui::{RichText, TextEdit, Ui};

pub struct FormState {
    prepay_date: NaiveDate,
    prepay_amount: Money,
    prepay_note: String,
    rate_date: NaiveDate,
    rate_pct: f64,
}

impl Default for FormState {
    fn default() -> Self {
        Self {
            prepay_date: today(),
            prepay_amount: Money::from_dollars(10_000.0),
            prepay_note: String::new(),
            rate_date: today(),
            rate_pct: 4.5,
        }
    }
}

impl FormState {
    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>, draft: &mut Mortgage, saved: &Mortgage, extra: &mut Money) {
        card(ui, "Details", tips::MORTGAGE_RATE, |ui| details(ui, cx, draft, saved));
        ui.add_space(6.0);
        card(ui, "Prepayments", tips::PREPAYMENT, |ui| self.prepayments(ui, cx, draft.id));
        ui.add_space(6.0);
        card(ui, "Rate changes / renewals", tips::RATE_CHANGE, |ui| self.rate_changes(ui, cx, draft.id));
        ui.add_space(6.0);
        card(ui, "What-if", tips::EXTRA_PAYMENT, |ui| {
            ui.horizontal(|ui| {
                ui.label("Extra per payment:");
                money_drag(ui, extra);
                if ui.small_button("Reset").clicked() {
                    *extra = Money::ZERO;
                }
            });
        });
    }

    fn prepayments(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>, mortgage_id: i64) {
        let list: Vec<&Prepayment> = cx.data.prepayments.iter().filter(|p| p.mortgage_id == mortgage_id).collect();
        egui::Grid::new("prepayments").striped(true).num_columns(4).show(ui, |ui| {
            for p in &list {
                let mut enabled = p.enabled;
                if ui.checkbox(&mut enabled, "").on_hover_text("Include in the projection").changed() {
                    let updated = Prepayment { enabled, ..(*p).clone() };
                    cx.out.report(cx.db.save_prepayment(&updated), |_| String::new());
                }
                ui.label(p.date.to_string());
                ui.label(RichText::new(p.amount.to_string()).monospace()).on_hover_text(&p.note);
                if ui.small_button("🗑").clicked() {
                    cx.out.report(cx.db.delete_prepayment(p.id), |_| "Prepayment removed".into());
                }
                ui.end_row();
            }
        });
        ui.horizontal(|ui| {
            date_edit(ui, "prepay-date", &mut self.prepay_date);
            money_drag(ui, &mut self.prepay_amount);
        });
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.prepay_note).hint_text("Note (optional)").desired_width(180.0));
            if ui.button("➕ Add prepayment").clicked() {
                let p = Prepayment {
                    id: 0,
                    mortgage_id,
                    date: self.prepay_date,
                    amount: self.prepay_amount.abs(),
                    note: self.prepay_note.trim().to_string(),
                    enabled: true,
                };
                if cx.out.report(cx.db.save_prepayment(&p), |_| "Prepayment added".into()).is_some() {
                    self.prepay_note.clear();
                }
            }
        });
    }

    fn rate_changes(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>, mortgage_id: i64) {
        egui::Grid::new("rate-changes").striped(true).num_columns(3).show(ui, |ui| {
            for r in cx.data.rate_changes.iter().filter(|r| r.mortgage_id == mortgage_id) {
                ui.label(r.effective_date.to_string());
                ui.label(format!("{:.2}%", r.annual_rate_pct));
                if ui.small_button("🗑").clicked() {
                    cx.out.report(cx.db.delete_rate_change(r.id), |_| "Rate change removed".into());
                }
                ui.end_row();
            }
        });
        ui.horizontal(|ui| {
            date_edit(ui, "rate-date", &mut self.rate_date);
            ui.add(egui::DragValue::new(&mut self.rate_pct).speed(0.01).range(0.0..=30.0).suffix(" %").fixed_decimals(2));
            if ui.button("➕ Add").clicked() {
                let r = RateChange { id: 0, mortgage_id, effective_date: self.rate_date, annual_rate_pct: self.rate_pct };
                cx.out.report(cx.db.add_rate_change(&r), |_| "Rate change added".into());
            }
        });
    }
}

fn details(ui: &mut Ui, cx: &mut PageCtx<'_>, d: &mut Mortgage, saved: &Mortgage) {
    egui::Grid::new("mortgage-form").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Name");
        ui.text_edit_singleline(&mut d.name);
        ui.end_row();
        ui.label("Original principal");
        money_drag(ui, &mut d.principal);
        ui.end_row();
        label_with_help(ui, "Interest rate", tips::MORTGAGE_RATE);
        ui.add(egui::DragValue::new(&mut d.annual_rate_pct).speed(0.01).range(0.0..=30.0).suffix(" %").fixed_decimals(2));
        ui.end_row();
        ui.label("Amortization");
        ui.horizontal(|ui| {
            let mut years = d.amortization_months / 12;
            let mut months = d.amortization_months % 12;
            let a = ui.add(egui::DragValue::new(&mut years).range(0..=40).suffix(" yr"));
            let b = ui.add(egui::DragValue::new(&mut months).range(0..=11).suffix(" mo"));
            if a.changed() || b.changed() {
                d.amortization_months = (years * 12 + months).max(1);
            }
        });
        ui.end_row();
        label_with_help(ui, "Start date", tips::MORTGAGE_START);
        date_edit(ui, "mortgage-start", &mut d.start_date);
        ui.end_row();
        label_with_help(ui, "Payment frequency", tips::MORTGAGE_FREQ);
        enum_combo(ui, "mortgage-freq", &mut d.frequency);
        ui.end_row();
        label_with_help(ui, "Payment override", tips::MORTGAGE_OVERRIDE);
        let mut over = d.payment_override.unwrap_or_default();
        money_drag(ui, &mut over);
        d.payment_override = (over.cents() > 0).then_some(over);
        ui.end_row();
        ui.label("Term");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut d.term_months).range(1..=120).suffix(" months"));
            let renewal = d.start_date + chrono::Months::new(d.term_months);
            ui.label(RichText::new(format!("renews {renewal}")).weak());
        });
        ui.end_row();
        ui.label("Notes");
        ui.add(TextEdit::multiline(&mut d.notes).desired_rows(2));
        ui.end_row();
    });
    let calc = regular_payment(d.frequency, d.annual_rate_pct, d.principal, d.amortization_months);
    ui.horizontal(|ui| {
        ui.label(format!("Calculated payment: {calc} {}", d.frequency.label().to_lowercase()));
        help(ui, "Payment at the starting rate. Rate changes recalculate it.");
    });
    let dirty = d != saved;
    ui.horizontal(|ui| {
        if ui.add_enabled(dirty, egui::Button::new("💾 Save details")).clicked() {
            cx.out.report(cx.db.save_mortgage(d), |_| "Mortgage saved".into());
        }
        if dirty {
            ui.label(RichText::new("Unsaved changes (projection already reflects them)").weak().small());
            if ui.small_button("Revert").clicked() {
                *d = saved.clone();
            }
        }
    });
}
