//! Step 2 (for unrecognised files): tell FinTrack which column holds what.

mod form;
mod view;

use super::{lock, pending};
use crate::error::AppResult;
use crate::extract::EditCtx;
use crate::flash;
use crate::views::components::heading;
use crate::views::{Nav, Page, hh_page};
use axum::extract::Path;
use axum::response::Response;
use axum_extra::extract::{Form, Query};
use fintrack_core::import::detect::{find_header_row, guess_spec};
use fintrack_core::import::reader::RawCsv;
use fintrack_core::model::CsvFormat;
use form::MappingForm;
use maud::html;
use serde::Deserialize;

const PREVIEW_ROWS: usize = 200;
const INTRO: &str = "Tell FinTrack which column holds what, then Preview. \
    Saved formats are recognised automatically next time.";

/// The form's starting state: the format being edited, a detected format
/// that didn't fit, or a guess from the file's contents.
fn initial(raw: &RawCsv, base: Option<&CsvFormat>) -> MappingForm {
    let stem = raw
        .file_name
        .trim_end_matches(".csv")
        .trim_end_matches(".CSV");
    if let Some(f) = base {
        let header = if f.spec.has_header {
            find_header_row(raw, &f.spec.header_signature)
        } else {
            None
        };
        if !f.spec.has_header || header.is_some() {
            return MappingForm::from_spec(&f.name, Some(f.id), &f.spec, header);
        }
    }
    let guess = guess_spec(raw);
    let mut form = MappingForm::from_spec(
        &format!("{stem} format"),
        None,
        &guess.spec,
        guess.header_row,
    );
    if let Some(f) = base {
        form.format_id = f.id.to_string();
        form.name = f.name.clone();
    }
    form
}

pub async fn show(
    h: EditCtx,
    Path((_, id)): Path<(String, String)>,
    Query(q): Query<MappingForm>,
) -> AppResult<Page> {
    let p = pending(&h, &id)?;
    let (preview, form) = {
        let g = lock(&p);
        let preview = RawCsv {
            file_name: g.raw.file_name.clone(),
            rows: g.raw.rows.iter().take(PREVIEW_ROWS).cloned().collect(),
        };
        let base = g.format.as_ref().or(g.mapping_base.as_ref());
        let form = if q.edited.is_empty() {
            initial(&g.raw, base)
        } else {
            q
        };
        (preview, form)
    };
    let spec = form.to_spec(&preview);
    let action = h.at(&format!("/import/{id}/mapping"));
    let cancel = html! {
        form method="post" action=(h.at(&format!("/import/{id}/cancel"))) class="inline" {
            button type="submit" { "Cancel import" }
        }
    };
    let body = html! {
        (heading(&format!("Map the columns of {}", preview.file_name), INTRO, cancel))
        section class="card" { (view::mapping_form(&action, &preview, &form)) }
        div id=(view::PREVIEW) class="refreshing" {
            (view::parsed_preview(&preview, &spec))
            (view::raw_preview(&preview, form.header_row()))
        }
    };
    Ok(hh_page(&h, Nav::Import, "Map columns", body))
}

#[derive(Deserialize)]
pub struct SaveForm {
    #[serde(flatten)]
    form: MappingForm,
    #[serde(default)]
    action: String,
}

pub async fn save(
    h: EditCtx,
    Path((_, id)): Path<(String, String)>,
    Form(f): Form<SaveForm>,
) -> AppResult<Response> {
    let p = pending(&h, &id)?;
    let retry = h.at(&format!("/import/{id}/mapping{}", f.form.to_query()));
    if f.form.name.trim().is_empty() {
        return Ok(flash::error(&h.st, &retry, "Give the format a name."));
    }
    let raw = lock(&p).raw.clone();
    let spec = f.form.to_spec(&raw);
    let existing = if f.action == "save_new" {
        None
    } else {
        f.form.format_id.parse::<i64>().ok()
    };
    let name = f.form.name.trim().to_string();
    let p2 = p.clone();
    let outcome =
        h.hh.run(move |db| {
            let builtin = existing.is_some_and(|id| {
                db.formats()
                    .is_ok_and(|fs| fs.iter().any(|f| f.id == id && f.builtin))
            });
            let mut format = CsvFormat {
                id: existing.unwrap_or(0),
                name,
                spec,
                builtin,
            };
            format.id = db.save_format(&format)?;
            Ok(lock(&p2).use_format(db, format))
        })
        .await?;
    Ok(match outcome {
        Ok(()) => flash::info(&h.st, &h.at(&format!("/import/{id}")), "Format saved."),
        Err(msg) => flash::error(&h.st, &retry, msg),
    })
}
