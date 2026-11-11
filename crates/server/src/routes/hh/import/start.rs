//! The Import page: upload files, demo samples, saved formats, import history.

use super::begin;
use super::session::{PendingImport, Upload};
use crate::error::{AppError, AppResult};
use crate::extract::{EditCtx, HhCtx};
use crate::flash;
use crate::households::SAMPLE_CSV_DIR;
use crate::views::components::{Num, Text, card, empty, heading, thead};
use crate::views::forms::action_button;
use crate::views::{Nav, Page, hh_page};
use axum::extract::{Multipart, Path};
use axum::response::Response;
use fintrack_core::db::ImportRecord;
use fintrack_core::help::tips;
use fintrack_core::model::CsvFormat;
use maud::{Markup, html};
use std::path::PathBuf;

const INTRO: &str =
    "Format and account are detected automatically; overlapping files are de-duplicated.";
const UNDO_CONFIRM: &str = "Delete every transaction added by this import?";

/// Sample CSVs shipped with a demo household, by file name.
fn samples(h: &HhCtx) -> (PathBuf, Vec<String>) {
    let dir = h.st.households.folder(&h.hh.info.slug).join(SAMPLE_CSV_DIR);
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map(|it| {
            it.flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.ends_with(".csv"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    (dir, names)
}

fn upload_card(h: &HhCtx) -> Markup {
    let body = html! {
        form method="post" action=(h.at("/import")) enctype="multipart/form-data"
            class="toolbar" {
            input type="file" name="files" accept=".csv,.txt,text/csv" multiple required;
            button type="submit" class="primary" { "Upload and continue" }
        }
        p class="muted" {
            "Choose one or more CSV exports from your bank. A copy of each imported file is "
            "kept in the household's imports folder on the server."
        }
    };
    card("Add files", tips::IMPORT, body)
}

fn samples_card(h: &HhCtx, names: &[String]) -> Markup {
    let body = html! {
        p {
            "This demo household was built from these bank exports. "
            "Import one again to see duplicate detection."
        }
        @for n in names {
            (action_button(&h.at(&format!("/import/samples/{n}")), n, "", None))
            " "
        }
    };
    card("Try the sample files", tips::DEMO_IMPORT, body)
}

fn formats_card(h: &HhCtx, formats: &[CsvFormat]) -> Markup {
    let delete = |f: &CsvFormat| h.at(&format!("/import/formats/{}/delete", f.id));
    let body = html! {
        p class="muted" {
            "Formats are recognised automatically. To change one, import a matching file "
            "and choose “Edit mapping”."
        }
        table {
            @for f in formats {
                tr {
                    td { (f.name) }
                    td class="muted" { @if f.builtin { "built-in" } @else { "custom" } }
                    td {
                        (action_button(&delete(f), "Delete", "small danger",
                            Some("Delete this format?")))
                    }
                }
            }
        }
    };
    card("Saved CSV formats", tips::FORMAT_DETECT, body)
}

fn history_card(h: &HhCtx, imports: &[ImportRecord]) -> Markup {
    let undo = |r: &ImportRecord| h.at(&format!("/import/history/{}/undo", r.id));
    let body = html! {
        @if imports.is_empty() {
            (empty("No imports yet."))
        } @else {
            div class="table-wrap" {
                table {
                    (thead(&[
                        ("When", Text), ("File", Text), ("Format", Text), ("Added", Num),
                        ("Duplicates", Num), ("", Text),
                    ]))
                    tbody {
                        @for r in imports {
                            tr {
                                td class="date" { (r.imported_at) }
                                td { (r.file_name) }
                                td { (r.format_name) }
                                td class="num" { (r.rows_inserted) }
                                td class="num" { (r.rows_duplicate) }
                                td {
                                    (action_button(&undo(r), "Undo", "small", Some(UNDO_CONFIRM)))
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    card("Import history", tips::UNDO_IMPORT, body)
}

pub async fn index(h: EditCtx) -> AppResult<Page> {
    let (formats, imports) = h.hh.run(|db| Ok((db.formats()?, db.imports()?))).await?;
    let (_, sample_names) = samples(&h);
    let body = html! {
        (heading("Import CSV", INTRO, html! {}))
        (upload_card(&h))
        @if !sample_names.is_empty() { (samples_card(&h, &sample_names)) }
        (history_card(&h, &imports))
        (formats_card(&h, &formats))
    };
    Ok(hh_page(&h, Nav::Import, "Import", body))
}

/// Start an import of `files` (the first now, the rest queued).
async fn start(h: &EditCtx, mut files: Vec<Upload>) -> AppResult<Response> {
    if files.is_empty() {
        return Ok(flash::error(
            &h.st,
            &h.at("/import"),
            "Choose at least one CSV file.",
        ));
    }
    let first = files.remove(0);
    let (hid, uid) = (h.hh.info.id, h.user.id);
    let started =
        h.hh.run(move |db| PendingImport::start(db, hid, uid, first, files))
            .await;
    match started {
        Ok(s) => Ok(begin(h, s)),
        Err(AppError::Internal(e)) => Ok(flash::error(
            &h.st,
            &h.at("/import"),
            format!("Couldn't read the file: {e:#}"),
        )),
        Err(e) => Err(e),
    }
}

pub async fn upload(h: EditCtx, mut form: Multipart) -> AppResult<Response> {
    let mut files = Vec::new();
    while let Some(part) = form
        .next_field()
        .await
        .map_err(|_| AppError::PayloadTooLarge)?
    {
        if part.name() != Some("files") {
            continue;
        }
        let file_name = part.file_name().unwrap_or("import.csv").to_string();
        let bytes = part
            .bytes()
            .await
            .map_err(|_| AppError::PayloadTooLarge)?
            .to_vec();
        if !bytes.is_empty() {
            files.push(Upload { file_name, bytes });
        }
    }
    start(&h, files).await
}

pub async fn sample(h: EditCtx, Path((_, name)): Path<(String, String)>) -> AppResult<Response> {
    let (dir, names) = samples(&h);
    if !names.contains(&name) {
        return Err(AppError::NotFound);
    }
    let bytes = std::fs::read(dir.join(&name))?;
    start(
        &h,
        vec![Upload {
            file_name: name,
            bytes,
        }],
    )
    .await
}

pub async fn undo(h: EditCtx, Path((_, id)): Path<(String, i64)>) -> AppResult<Response> {
    h.hh.run(move |db| db.undo_import(id)).await?;
    Ok(flash::info(
        &h.st,
        &h.at("/import"),
        "Import undone; its transactions were removed.",
    ))
}

pub async fn delete_format(h: EditCtx, Path((_, id)): Path<(String, i64)>) -> AppResult<Response> {
    h.hh.run(move |db| db.delete_format(id)).await?;
    Ok(flash::info(&h.st, &h.at("/import"), "Format deleted."))
}
