//! Base path, sign-in gate and static assets.

mod common;

use axum::http::StatusCode;
use common::{TestApp, url};

#[tokio::test]
async fn health_and_assets_are_public() {
    let mut app = TestApp::new();
    assert_eq!(app.get(&url("/healthz")).await.body, "ok");
    let css = app.get(&url("/static/app.css")).await;
    assert_eq!(css.status, StatusCode::OK);
    assert!(css.body.contains("light-dark("));
    for script in ["htmx.min.js", "chart.min.js", "app.js", "charts.js"] {
        let r = app.get(&url(&format!("/static/{script}"))).await;
        assert_eq!(r.status, StatusCode::OK, "{script}");
    }
    assert_eq!(
        app.get(&url("/static/nope.js")).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn everything_lives_under_the_base_path() {
    let mut app = TestApp::new();
    assert_eq!(app.get("/").await.redirect(), "/finances");
    assert_eq!(app.get("/healthz").await.status, StatusCode::NOT_FOUND);
    for path in ["/finances", "/finances/"] {
        let r = app.get(path).await;
        assert!(
            r.redirect().starts_with("/finances/login?next="),
            "{path} → {:?}",
            r.location
        );
    }
}

#[tokio::test]
async fn pages_need_sign_in_and_links_carry_the_base_path() {
    let mut app = TestApp::new();
    let slug = app.with_admin_and_household().await;
    let mut anon = app.client();
    let r = anon.get(&url(&format!("/h/{slug}/transactions?q=x"))).await;
    assert_eq!(
        r.redirect(),
        "/finances/login?next=%2Ffinances%2Fh%2Fhome%2Ftransactions%3Fq%3Dx"
    );

    // The admin lands on the only household.
    assert_eq!(
        app.get(&url("/")).await.redirect(),
        format!("/finances/h/{slug}")
    );
    for path in ["", "/", "/transactions", "/budgets", "/import", "/help"] {
        let r = app.get(&url(&format!("/h/{slug}{path}"))).await;
        assert_eq!(r.status, StatusCode::OK, "{path}: {}", r.body);
        for href in r
            .body
            .split("href=\"")
            .skip(1)
            .map(|s| s.split('"').next().unwrap())
        {
            assert!(
                href.starts_with("/finances")
                    || href.starts_with('#')
                    || href.starts_with("javascript:"),
                "{path}: link {href} escapes the base path"
            );
        }
    }
}
