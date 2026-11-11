//! Importing bank CSV files: upload → (map columns) → review accounts and
//! duplicates → commit. Also import history (undo) and saved formats.

mod mapping;
mod review;
mod session;
mod start;

pub use session::PendingImport;

use crate::error::{AppError, AppResult};
use crate::extract::EditCtx;
use crate::flash;
use crate::state::Shared;
use axum::Router;
use axum::response::Response;
use axum::routing::{get, post};
use std::sync::{Arc, Mutex};

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/import", get(start::index).post(start::upload))
        .route("/import/samples/{name}", post(start::sample))
        .route("/import/history/{id}/undo", post(start::undo))
        .route("/import/formats/{id}/delete", post(start::delete_format))
        .route("/import/{id}", get(review::show))
        .route(
            "/import/{id}/mapping",
            get(mapping::show).post(mapping::save),
        )
        .route(
            "/import/{id}/groups/{group}/account",
            post(review::set_account),
        )
        .route(
            "/import/{id}/groups/{group}/new-account",
            post(review::new_account),
        )
        .route("/import/{id}/commit", post(review::commit))
        .route("/import/{id}/next", post(review::next))
        .route("/import/{id}/cancel", post(review::cancel))
}

/// The pending import `id`, if it belongs to this household and user.
fn pending(h: &EditCtx, id: &str) -> AppResult<Arc<Mutex<PendingImport>>> {
    let p = h.st.pending.get(id).ok_or(AppError::NotFound)?;
    let ok = {
        let g = p.lock().unwrap_or_else(|e| e.into_inner());
        g.household_id == h.hh.info.id && g.user_id == h.user.id
    };
    if ok { Ok(p) } else { Err(AppError::NotFound) }
}

fn lock(p: &Mutex<PendingImport>) -> std::sync::MutexGuard<'_, PendingImport> {
    p.lock().unwrap_or_else(|e| e.into_inner())
}

/// Where the wizard continues for import `id`, with an optional message.
fn continue_at(h: &EditCtx, id: &str, needs_mapping: bool, message: Option<String>) -> Response {
    let to = h.at(&format!(
        "/import/{id}{}",
        if needs_mapping { "/mapping" } else { "" }
    ));
    match message {
        Some(m) => flash::error(&h.st, &to, m),
        None => flash::redirect(&to),
    }
}

/// Store a newly started import and send the user to its first step.
fn begin(h: &EditCtx, started: (PendingImport, Option<String>)) -> Response {
    let (p, message) = started;
    let needs_mapping = p.format.is_none();
    let id = h.st.pending.insert(p);
    continue_at(h, &id, needs_mapping, message)
}
