//! URL routing. Everything is mounted under the configured base path.
//!
//! ```text
//! /healthz, /static/*              public
//! /login, /logout, /setup          sign-in (public)
//! /, /households, /account, /help  signed in
//! /admin/*                         admins
//! /h/{hh}/*                        members of household {hh}
//! ```

pub mod account;
pub mod admin;
pub mod hh;
pub mod home;
pub mod signin;

use crate::state::Shared;
use crate::{assets, auth, flash};
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderName, HeaderValue, header};
use axum::middleware::{from_fn, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tower::Layer;
use tower_http::compression::CompressionLayer;
use tower_http::normalize_path::{NormalizePath, NormalizePathLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

/// Content Security Policy: only this server's own scripts, styles and images.
const CSP: &str = concat!(
    "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; ",
    "img-src 'self' data:; form-action 'self'; frame-ancestors 'none'; base-uri 'none'",
);

fn default_header(name: HeaderName, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::if_not_present(name, HeaderValue::from_static(value))
}

/// A file download response.
pub fn attachment(bytes: Vec<u8>, content_type: &'static str, file_name: &str) -> Response {
    let mut resp = bytes.into_response();
    let headers = resp.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    let safe = fintrack_core::db::safe_file_name(file_name);
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{safe}\"")) {
        headers.insert(header::CONTENT_DISPOSITION, v);
    }
    resp
}

/// The complete app: routes behind trailing-slash normalisation, so
/// `/finances`, `/finances/` and `/finances/h/home/` all resolve.
pub type App = NormalizePath<Router>;

pub fn router(st: Shared) -> App {
    NormalizePathLayer::trim_trailing_slash().layer(routes(st))
}

fn routes(st: Shared) -> Router {
    let signed_in = Router::new()
        .route("/", get(home::root))
        .route("/households", get(home::picker))
        .route("/help", get(hh::help::global))
        .merge(account::routes())
        .merge(admin::routes())
        .nest("/h/{hh}", hh::routes())
        .route_layer(from_fn_with_state(st.clone(), auth::require_login));

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/static/{name}", get(assets::serve))
        .merge(signin::routes())
        .merge(signed_in)
        .layer(from_fn_with_state(st.clone(), flash::layer))
        .layer(from_fn(auth::origin::guard))
        .layer(DefaultBodyLimit::max(st.cfg.max_upload_bytes))
        .layer(default_header(header::CONTENT_SECURITY_POLICY, CSP))
        .layer(default_header(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(default_header(header::REFERRER_POLICY, "same-origin"))
        .layer(default_header(header::X_FRAME_OPTIONS, "DENY"))
        // Finance pages must never be cached by the browser or a proxy.
        .layer(default_header(header::CACHE_CONTROL, "no-store"))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(st.clone());

    let base = st.cfg.base_path.clone();
    if base.is_empty() {
        return app;
    }
    let home = base.clone();
    Router::new()
        .route("/", get(move || async move { flash::redirect(&home) }))
        .nest(&base, app)
}
