//! Mortgage inputs: details, prepayments, rate changes and the what-if extra payment.

use super::{Loaded, ProjectionQuery};
use crate::error::AppResult;
use crate::extract::{EditCtx, HhCtx};
use crate::flash;
use crate::form::{date, key, money, number, opt_money, text};
use crate::views::components::{card, tip};
use crate::views::format::{date as date_text, money_plain};
use crate::views::forms::{
    action_button, date_input, enum_select, field, number_input, text_input,
};
use axum::Form;
use axum::extract::Path;
use axum::response::Response;
use fintrack_core::help::tips;
use fintrack_core::model::{Mortgage, Prepayment, RateChange};
use fintrack_core::mortgage::regular_payment;
use maud::{Markup, html};
use serde::Deserialize;

fn details(h: &HhCtx, m: &Mortgage) -> Markup {
    let calc = regular_payment(
        m.frequency,
        m.annual_rate_pct,
        m.principal,
        m.amortization_months,
    );
    let renewal = format!(
        "Term (months), renews {}",
        date_text(m.start_date + chrono::Months::new(m.term_months))
    );
    let override_value = m.payment_override.map(|p| p.to_plain()).unwrap_or_default();
    let ro = !h.hh.can_edit();
    html! {
        form method="post" action=(h.at(&format!("/mortgage/{}", m.id))) class="form-grid" {
            fieldset disabled[ro] class="form-grid plain" {
                (field("Name", "", text_input("name", &m.name)))
                (field("Original principal", "", text_input("principal", &m.principal.to_plain())))
                (field("Interest rate (%)", tips::MORTGAGE_RATE,
                    number_input("rate", m.annual_rate_pct, ("0", "30"), "0.01")))
                (field("Amortization (years)", "",
                    number_input("years", m.amortization_months / 12, ("0", "40"), "1")))
                (field("…and months", "",
                    number_input("months", m.amortization_months % 12, ("0", "11"), "1")))
                (field("Start date", tips::MORTGAGE_START,
                    date_input("start", &m.start_date.to_string())))
                (field("Payment frequency", tips::MORTGAGE_FREQ,
                    enum_select("frequency", m.frequency, false)))
                (field("Payment override", tips::MORTGAGE_OVERRIDE,
                    text_input("payment_override", &override_value)))
                (field(&renewal, "", number_input("term", m.term_months, ("1", "120"), "1")))
                (field("Notes", "", html! { textarea name="notes" rows="2" { (m.notes) } }))
            }
            p class="muted" {
                "Calculated payment: " (money_plain(calc)) " "
                (m.frequency.label().to_lowercase()) " "
                (tip("Payment at the starting rate. Rate changes recalculate it."))
            }
            @if !ro { div { button type="submit" class="primary" { "Save details" } } }
        }
    }
}

fn prepayments(h: &HhCtx, l: &Loaded) -> Markup {
    let base = format!("/mortgage/{}/prepayments", l.m.id);
    html! {
        table {
            @for p in &l.prepayments {
                tr {
                    td { @if p.enabled { "✔" } @else { span class="muted" { "off" } } }
                    td class="date" { (date_text(p.date)) }
                    td class="num" { (money_plain(p.amount)) }
                    td class="muted" { (p.note) }
                    td {
                        @if h.hh.can_edit() {
                            @let at = |action: &str| h.at(&format!("{base}/{}/{action}", p.id));
                            @let toggle = if p.enabled { "Exclude" } else { "Include" };
                            (action_button(&at("toggle"), toggle, "small", None))
                            " "
                            (action_button(&at("delete"), "Remove", "small danger", None))
                        }
                    }
                }
            }
        }
        @if h.hh.can_edit() {
            form method="post" action=(h.at(&base)) class="toolbar" {
                (field("Date", "", date_input("date", "")))
                (field("Amount", "", text_input("amount", "10000.00")))
                (field("Note", "", text_input("note", "")))
                button type="submit" { "Add prepayment" }
            }
        }
    }
}

fn rate_changes(h: &HhCtx, l: &Loaded) -> Markup {
    let base = format!("/mortgage/{}/rates", l.m.id);
    html! {
        table {
            @for r in &l.rates {
                tr {
                    td class="date" { (date_text(r.effective_date)) }
                    td class="num" { (format!("{:.2}%", r.annual_rate_pct)) }
                    td {
                        @if h.hh.can_edit() {
                            @let delete = h.at(&format!("{base}/{}/delete", r.id));
                            (action_button(&delete, "Remove", "small danger", None))
                        }
                    }
                }
            }
        }
        @if h.hh.can_edit() {
            form method="post" action=(h.at(&base)) class="toolbar" {
                (field("From", "", date_input("date", "")))
                (field("Rate (%)", "", number_input("rate", "4.50", ("0", "30"), "0.01")))
                button type="submit" { "Add rate change" }
            }
        }
    }
}

fn what_if(h: &HhCtx, l: &Loaded, q: &ProjectionQuery) -> Markup {
    let page = h.at(&format!("/mortgage/{}", l.m.id));
    html! {
        form method="get" action=(page) class="toolbar" {
            input type="hidden" name="view" value=(q.view);
            (field("Extra per payment", tips::EXTRA_PAYMENT, text_input("extra", &q.extra)))
            button type="submit" { "Show" }
            @if !q.extra.is_empty() { a class="btn" href=(page) { "Reset" } }
        }
    }
}

pub fn inputs(h: &HhCtx, l: &Loaded, q: &ProjectionQuery) -> Markup {
    html! {
        (card("Details", tips::MORTGAGE_RATE, details(h, &l.m)))
        (card("Prepayments", tips::PREPAYMENT, prepayments(h, l)))
        (card("Rate changes / renewals", tips::RATE_CHANGE, rate_changes(h, l)))
        (card("What-if", tips::EXTRA_PAYMENT, what_if(h, l, q)))
    }
}

fn back(h: &EditCtx, id: i64) -> String {
    h.at(&format!("/mortgage/{id}"))
}

#[derive(Deserialize)]
pub struct DetailsForm {
    name: String,
    principal: String,
    rate: String,
    years: String,
    months: String,
    start: String,
    frequency: String,
    #[serde(default)]
    payment_override: String,
    term: String,
    #[serde(default)]
    notes: String,
}

pub async fn save_details(
    h: EditCtx,
    Path((_, id)): Path<(String, i64)>,
    Form(f): Form<DetailsForm>,
) -> AppResult<Response> {
    let years: u32 = number(&f.years, "Amortization years")?;
    let months: u32 = number(&f.months, "Amortization months")?;
    let m = Mortgage {
        id,
        name: text(&f.name, "Name")?,
        principal: money(&f.principal, "Principal")?.abs(),
        annual_rate_pct: number(&f.rate, "Interest rate")?,
        amortization_months: (years * 12 + months).max(1),
        start_date: date(&f.start, "Start date")?,
        frequency: key(&f.frequency, "payment frequency")?,
        payment_override: opt_money(&f.payment_override, "Payment override")?
            .filter(|p| p.cents() > 0),
        term_months: number::<u32>(&f.term, "Term")?.max(1),
        notes: f.notes.trim().to_string(),
    };
    h.hh.run(move |db| db.save_mortgage(&m)).await?;
    Ok(flash::info(&h.st, &back(&h, id), "Mortgage saved."))
}

#[derive(Deserialize)]
pub struct PrepaymentForm {
    date: String,
    amount: String,
    #[serde(default)]
    note: String,
}

pub async fn add_prepayment(
    h: EditCtx,
    Path((_, id)): Path<(String, i64)>,
    Form(f): Form<PrepaymentForm>,
) -> AppResult<Response> {
    let p = Prepayment {
        id: 0,
        mortgage_id: id,
        date: date(&f.date, "Date")?,
        amount: money(&f.amount, "Amount")?.abs(),
        note: f.note.trim().to_string(),
        enabled: true,
    };
    h.hh.run(move |db| db.save_prepayment(&p)).await?;
    Ok(flash::info(&h.st, &back(&h, id), "Prepayment added."))
}

pub async fn toggle_prepayment(
    h: EditCtx,
    Path((_, id, item)): Path<(String, i64, i64)>,
) -> AppResult<Response> {
    h.hh.run(move |db| {
        if let Some(p) = db.prepayments(id)?.into_iter().find(|p| p.id == item) {
            db.save_prepayment(&Prepayment {
                enabled: !p.enabled,
                ..p
            })?;
        }
        Ok(())
    })
    .await?;
    Ok(flash::redirect(&back(&h, id)))
}

pub async fn delete_prepayment(
    h: EditCtx,
    Path((_, id, item)): Path<(String, i64, i64)>,
) -> AppResult<Response> {
    h.hh.run(move |db| db.delete_prepayment(item)).await?;
    Ok(flash::info(&h.st, &back(&h, id), "Prepayment removed."))
}

#[derive(Deserialize)]
pub struct RateForm {
    date: String,
    rate: String,
}

pub async fn add_rate(
    h: EditCtx,
    Path((_, id)): Path<(String, i64)>,
    Form(f): Form<RateForm>,
) -> AppResult<Response> {
    let r = RateChange {
        id: 0,
        mortgage_id: id,
        effective_date: date(&f.date, "Date")?,
        annual_rate_pct: number(&f.rate, "Rate")?,
    };
    h.hh.run(move |db| db.add_rate_change(&r)).await?;
    Ok(flash::info(&h.st, &back(&h, id), "Rate change added."))
}

pub async fn delete_rate(
    h: EditCtx,
    Path((_, id, item)): Path<(String, i64, i64)>,
) -> AppResult<Response> {
    h.hh.run(move |db| db.delete_rate_change(item)).await?;
    Ok(flash::info(&h.st, &back(&h, id), "Rate change removed."))
}
