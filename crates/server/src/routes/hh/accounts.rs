//! Bank accounts: how they're recognised in CSV files, opening balances, archiving.

use super::data::{Data, today};
use crate::error::{AppError, AppResult};
use crate::extract::{EditCtx, HhCtx};
use crate::flash;
use crate::form::{checked, key, money, opt_id, text};
use crate::state::Shared;
use crate::views::components::{Num, Text, card, empty, heading, link_button, thead};
use crate::views::format::{date, money as money_cell};
use crate::views::forms::{action_button, checkbox, enum_select, field, text_input};
use crate::views::{Nav, Page, hh_page};
use axum::extract::Path;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Form, Router};
use fintrack_core::analytics::balances::balance_on;
use fintrack_core::help::tips;
use fintrack_core::model::{Account, CsvFormat};
use maud::{Markup, html};
use serde::Deserialize;

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/accounts", get(list).post(create))
        .route("/accounts/new", get(new))
        .route("/accounts/{id}", get(edit).post(update))
        .route("/accounts/{id}/delete", post(delete))
}

fn account_row(h: &HhCtx, a: &Account, data: &Data) -> Markup {
    let own: Vec<_> = data.txns.iter().filter(|t| t.account_id == a.id).collect();
    let span = match (
        own.iter().map(|t| t.date).min(),
        own.iter().map(|t| t.date).max(),
    ) {
        (Some(lo), Some(hi)) => format!("{} – {} ({})", date(lo), date(hi), own.len()),
        _ => "no transactions".into(),
    };
    let ids: Vec<&str> = [a.account_number.as_str(), a.filename_hint.as_str()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    html! {
        tr {
            td { strong { (a.name) } @if a.archived { span class="muted" { " (archived)" } } }
            td { (a.kind.label()) }
            td { (a.institution) }
            td class="muted" { @if ids.is_empty() { "—" } @else { (ids.join(" · ")) } }
            td class="muted" { (span) }
            td class="num" { (money_cell(balance_on(a, &data.txns, today()))) }
            td {
                @if h.hh.can_edit() {
                    a class="btn small" href=(h.at(&format!("/accounts/{}", a.id))) { "Edit" }
                }
            }
        }
    }
}

async fn list(h: HhCtx) -> AppResult<Page> {
    let data = h.hh.run(Data::load).await?;
    let body = html! {
        (heading("Accounts", tips::ACCOUNT_DETECT, html! {
            @if h.hh.can_edit() { (link_button(&h.at("/accounts/new"), "New account", true)) }
        }))
        (card("", "", html! {
            @if data.accounts.is_empty() {
                (empty("No accounts yet. Create one here, or while importing your first CSV."))
            } @else {
                div class="table-wrap" {
                    table {
                        (thead(&[
                            ("Name", Text), ("Type", Text), ("Bank", Text), ("Recognised by", Text),
                            ("Transactions", Text), ("Balance", Num), ("", Text),
                        ]))
                        tbody { @for a in &data.accounts { (account_row(&h, a, &data)) } }
                    }
                }
            }
        }))
    };
    Ok(hh_page(&h, Nav::Accounts, "Accounts", body))
}

fn editor(h: &EditCtx, a: &Account, formats: &[CsvFormat]) -> Page {
    let (action, title) = if a.id == 0 {
        (h.at("/accounts"), "New account")
    } else {
        (h.at(&format!("/accounts/{}", a.id)), "Edit account")
    };
    let body = html! {
        (heading(title, "", html! { a class="btn" href=(h.at("/accounts")) { "← Accounts" } }))
        (card("", "", html! {
            form method="post" action=(action) class="form-grid" {
                (field("Name", "", text_input("name", &a.name)))
                (field("Bank / institution", "", text_input("institution", &a.institution)))
                (field("Type", "", enum_select("kind", a.kind, false)))
                (field("Account number", tips::ACCOUNT_NUMBER,
                    text_input("account_number", &a.account_number)))
                (field("Filename hint", tips::FILENAME_HINT,
                    text_input("filename_hint", &a.filename_hint)))
                (field("Usual CSV format", "", html! {
                    select name="format" {
                        option value="" { "(any)" }
                        @for f in formats {
                            option value=(f.id) selected[a.format_id == Some(f.id)] { (f.name) }
                        }
                    }
                }))
                (field("Opening balance", tips::OPENING_BALANCE,
                    text_input("opening_balance", &a.opening_balance.to_plain())))
                (checkbox("archived", a.archived, html! { "Archived" }))
                div { button type="submit" class="primary" { "Save" } }
            }
        }))
        @if a.id != 0 {
            (card("Delete", "", html! {
                p { "Deleting an account also deletes all of its transactions." }
                (action_button(&h.at(&format!("/accounts/{}/delete", a.id)), "Delete account",
                    "danger", Some("Delete this account and all its transactions?")))
            }))
        }
    };
    hh_page(h, Nav::Accounts, title, body)
}

async fn new(h: EditCtx) -> AppResult<Page> {
    let formats = h.hh.run(|db| db.formats()).await?;
    Ok(editor(&h, &Account::default(), &formats))
}

async fn edit(h: EditCtx, Path((_, id)): Path<(String, i64)>) -> AppResult<Page> {
    let (accounts, formats) = h.hh.run(|db| Ok((db.accounts()?, db.formats()?))).await?;
    let a = accounts
        .into_iter()
        .find(|a| a.id == id)
        .ok_or(AppError::NotFound)?;
    Ok(editor(&h, &a, &formats))
}

#[derive(Deserialize)]
pub struct AccountForm {
    name: String,
    #[serde(default)]
    institution: String,
    kind: String,
    #[serde(default)]
    account_number: String,
    #[serde(default)]
    filename_hint: String,
    #[serde(default)]
    format: String,
    opening_balance: String,
    archived: Option<String>,
}

impl AccountForm {
    fn account(&self, id: i64) -> AppResult<Account> {
        Ok(Account {
            id,
            name: text(&self.name, "Account name")?,
            institution: self.institution.trim().into(),
            kind: key(&self.kind, "account type")?,
            account_number: self.account_number.trim().into(),
            filename_hint: self.filename_hint.trim().into(),
            format_id: opt_id(&self.format),
            opening_balance: if self.opening_balance.trim().is_empty() {
                Default::default()
            } else {
                money(&self.opening_balance, "Opening balance")?
            },
            archived: checked(&self.archived),
        })
    }
}

async fn save(h: &EditCtx, a: Account) -> AppResult<Response> {
    h.hh.run(move |db| db.save_account(&a)).await?;
    Ok(flash::info(&h.st, &h.at("/accounts"), "Account saved."))
}

async fn create(h: EditCtx, Form(f): Form<AccountForm>) -> AppResult<Response> {
    let a = f.account(0)?;
    save(&h, a).await
}

async fn update(
    h: EditCtx,
    Path((_, id)): Path<(String, i64)>,
    Form(f): Form<AccountForm>,
) -> AppResult<Response> {
    let a = f.account(id)?;
    save(&h, a).await
}

async fn delete(h: EditCtx, Path((_, id)): Path<(String, i64)>) -> AppResult<Response> {
    h.hh.run(move |db| db.delete_account(id)).await?;
    Ok(flash::info(&h.st, &h.at("/accounts"), "Account deleted."))
}
