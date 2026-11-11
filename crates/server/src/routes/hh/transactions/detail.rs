//! One transaction: everything known about it, plus its note.

use crate::error::{AppError, AppResult};
use crate::extract::HhCtx;
use crate::routes::hh::back_or;
use crate::routes::hh::data::Data;
use crate::views::components::{card, heading};
use crate::views::format::{date, money};
use crate::views::{Nav, Page, hh_page};
use axum::extract::{Path, Query};
use maud::html;
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct BackQuery {
    back: String,
}

pub async fn show(
    h: HhCtx,
    Path((_, id)): Path<(String, i64)>,
    Query(q): Query<BackQuery>,
) -> AppResult<Page> {
    let data = h.hh.run(Data::load).await?;
    let t = data
        .txns
        .iter()
        .find(|t| t.id == id)
        .ok_or(AppError::NotFound)?;
    let back = back_or(&h, &q.back, "/transactions");
    let rule = t
        .rule_id
        .and_then(|r| data.rules.iter().find(|x| x.id == r));
    let cat = |c: Option<i64>| c.map_or_else(|| "—".to_string(), |c| data.cats.full_name(c));
    let rule_text = rule.map_or_else(|| "—".to_string(), |r| r.describe());
    let body = html! {
        (heading(&t.description, "", html! { a class="btn" href=(back) { "← Back" } }))
        div class="grid" {
            (card("Details", "", html! {
                table {
                    tr { th { "Date" } td { (date(t.date)) } }
                    tr { th { "Account" } td { (data.account_name(t.account_id)) } }
                    tr { th { "Amount" } td { (money(t.amount)) } }
                    tr { th { "Description" } td { (t.description) } }
                    tr { th { "Memo" } td { (t.memo) } }
                    @if let Some(b) = t.balance { tr { th { "Bank balance" } td { (money(b)) } } }
                    tr { th { "Category" } td { (cat(t.category_id())) } }
                    tr { th { "From rule" } td { (rule_text) } }
                    tr { th { "Rule category" } td { (cat(t.auto_category_id)) } }
                    tr { th { "Manual override" } td { (cat(t.manual_category_id)) } }
                }
            }))
            (card("Note", "A private note, searchable from the Transactions page.", html! {
                @if h.hh.can_edit() {
                    form method="post" action=(h.at(&format!("/transactions/{id}/note")))
                        class="form-stack" {
                        input type="hidden" name="back" value=(back);
                        textarea name="note" rows="4" { (t.note) }
                        div { button type="submit" class="primary" { "Save note" } }
                    }
                } @else {
                    p { (t.note) }
                }
            }))
        }
    };
    Ok(hh_page(&h, Nav::Transactions, "Transaction", body))
}
