//! Changes to transactions: category overrides, notes, bulk actions.

use super::table;
use crate::error::{AppError, AppResult};
use crate::extract::EditCtx;
use crate::flash;
use crate::form::opt_id;
use crate::htmx;
use crate::routes::hh::back_or;
use crate::routes::hh::data::Data;
use axum::Form;
use axum::extract::Path;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Response};
use axum_extra::extract::Form as MultiForm;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CategoryForm {
    category: String,
    #[serde(default)]
    back: String,
}

/// Set (or with an empty value, clear) the manual category of one transaction.
/// htmx gets the updated table row back; a plain form post is redirected.
pub async fn set_category(
    h: EditCtx,
    Path((_, id)): Path<(String, i64)>,
    headers: HeaderMap,
    Form(f): Form<CategoryForm>,
) -> AppResult<Response> {
    let category = opt_id(&f.category);
    let data =
        h.hh.run(move |db| {
            db.set_manual_category(&[id], category)?;
            Data::load(db)
        })
        .await?;
    let back = back_or(&h, &f.back, "/transactions");
    if !htmx::wants_fragment(&headers) {
        return Ok(flash::redirect(&back));
    }
    let t = data
        .txns
        .iter()
        .find(|t| t.id == id)
        .ok_or(AppError::NotFound)?;
    let page = back.split('#').next().unwrap_or_default();
    Ok(Html(table::row(&h, t, &data, page).into_string()).into_response())
}

#[derive(Deserialize)]
pub struct NoteForm {
    note: String,
    #[serde(default)]
    back: String,
}

pub async fn set_note(
    h: EditCtx,
    Path((_, id)): Path<(String, i64)>,
    Form(f): Form<NoteForm>,
) -> AppResult<Response> {
    let note = f.note.trim().to_string();
    h.hh.run(move |db| db.set_note(id, &note)).await?;
    Ok(flash::info(
        &h.st,
        &back_or(&h, &f.back, "/transactions"),
        "Note saved.",
    ))
}

#[derive(Deserialize)]
pub struct BulkForm {
    #[serde(default)]
    ids: Vec<i64>,
    action: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    back: String,
}

pub async fn bulk(h: EditCtx, MultiForm(f): MultiForm<BulkForm>) -> AppResult<Response> {
    let back = back_or(&h, &f.back, "/transactions");
    if f.ids.is_empty() {
        return Ok(flash::error(&h.st, &back, "Tick some transactions first."));
    }
    let n = f.ids.len();
    let ids = f.ids;
    let message = match f.action.as_str() {
        "set" => {
            let Some(category) = opt_id(&f.category) else {
                return Ok(flash::error(&h.st, &back, "Choose a category to set."));
            };
            h.hh.run(move |db| db.set_manual_category(&ids, Some(category)))
                .await?;
            format!("Set the category of {n} transactions.")
        }
        "clear" => {
            h.hh.run(move |db| db.set_manual_category(&ids, None))
                .await?;
            format!("Cleared overrides on {n} transactions.")
        }
        "delete" => {
            h.hh.run(move |db| db.delete_transactions(&ids)).await?;
            format!("Deleted {n} transactions.")
        }
        other => return Err(AppError::bad_request(format!("Unknown action “{other}”."))),
    };
    Ok(flash::info(&h.st, &back, message))
}
