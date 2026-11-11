//! Every page renders for a demo household, and common edits work.

mod common;

use axum::http::StatusCode;
use common::{TestApp, url};

#[tokio::test]
async fn every_page_renders() {
    let mut app = TestApp::new();
    let slug = app.with_admin_and_household().await;
    let mortgage = app
        .get(&url(&format!("/h/{slug}/mortgage")))
        .await
        .redirect()
        .to_string();
    let mut paths: Vec<String> = [
        "",
        "/transactions",
        "/transactions?cat=none&q=coffee",
        "/transactions?range=custom&from=2025-01-01",
        "/budgets",
        "/budgets?g=quarter",
        "/import",
        "/categorize",
        "/categorize/rules",
        "/categorize/rules?test=STARBUCKS&amount=-5",
        "/categorize/rules/new?pattern=SHELL",
        "/categories",
        "/accounts",
        "/accounts/new",
        "/analytics/cash-flow",
        "/analytics/breakdown?by=sub&g=quarter",
        "/analytics/trends",
        "/analytics/payees?flow=income",
        "/analytics/recurring",
        "/analytics/balances",
        "/help",
    ]
    .iter()
    .map(|p| url(&format!("/h/{slug}{p}")))
    .collect();
    paths.extend([mortgage.clone(), format!("{mortgage}?view=all&extra=200")]);
    paths.extend(
        [
            "/households",
            "/account",
            "/help",
            "/admin/users",
            "/admin/households",
            "/admin/access",
        ]
        .map(url),
    );
    for p in &paths {
        let r = app.get(p).await;
        assert_eq!(
            r.status,
            StatusCode::OK,
            "{p}: {}",
            &r.body[..r.body.len().min(500)]
        );
        assert!(r.body.contains("</html>"), "{p}");
    }
    // Charts are a canvas plus their data as JSON (and a fallback table).
    let page = app
        .get(&url(&format!("/h/{slug}/analytics/breakdown")))
        .await
        .body;
    assert!(page.contains("Show as table"));
    let attr = page
        .split("<canvas data-chart=\"")
        .nth(1)
        .expect("chart canvas");
    let json = attr[..attr.find('"').unwrap()]
        .replace("&quot;", "\"")
        .replace("&amp;", "&");
    let spec: serde_json::Value = serde_json::from_str(&json).expect("valid chart JSON");
    assert_eq!(spec["stacked"], true);
    // Clicking a bar opens that group's transactions for that period.
    let link = spec["bars"][0]["links"][0].as_str().expect("bar link");
    assert!(link.starts_with(&url(&format!("/h/{slug}/transactions?range=custom"))));
    // Pages load htmx and Chart.js, and are boosted.
    assert!(page.contains("htmx.min.js") && page.contains("chart.min.js"));
    assert!(page.contains("hx-boost=\"true\""));
}

#[tokio::test]
async fn budgets_rules_and_overrides() {
    let mut app = TestApp::new();
    let slug = app.with_admin_and_household().await;
    let h = |p: &str| url(&format!("/h/{slug}{p}"));
    let st = app.st.clone();
    let db = st.households.get(&slug).unwrap();
    let (cat, txn) = {
        let db = db.lock().unwrap();
        let cat = db
            .categories()
            .unwrap()
            .iter()
            .find(|c| c.name == "Shopping")
            .unwrap()
            .id;
        (cat, db.transactions().unwrap()[0].id)
    };

    let r = app
        .post(
            &h("/budgets"),
            &[("category", &cat.to_string()), ("amount", "250")],
        )
        .await;
    assert!(r.status.is_redirection());
    assert!(
        db.lock()
            .unwrap()
            .budgets()
            .unwrap()
            .iter()
            .any(|b| b.category_id == cat)
    );

    let rule = [
        ("edited", "1"),
        ("field", "description"),
        ("match_kind", "contains"),
        ("pattern", "AMAZON"),
        ("direction", "any"),
        ("category", &cat.to_string()),
        ("priority", "50"),
        ("enabled", "1"),
    ];
    let r = app.post(&h("/categorize/rules"), &rule).await;
    assert!(
        r.redirect().ends_with("/categorize/rules"),
        "{:?}",
        r.location
    );
    assert!(
        db.lock()
            .unwrap()
            .rules()
            .unwrap()
            .iter()
            .any(|r| r.pattern == "AMAZON")
    );

    let r = app
        .post(
            &h(&format!("/transactions/{txn}/category")),
            &[("category", &cat.to_string())],
        )
        .await;
    assert!(r.status.is_redirection());
    let t = db
        .lock()
        .unwrap()
        .transactions()
        .unwrap()
        .into_iter()
        .find(|t| t.id == txn)
        .unwrap();
    assert_eq!(t.manual_category_id, Some(cat));

    // htmx asks for just the updated row.
    let row = app
        .post_htmx(
            &h(&format!("/transactions/{txn}/category")),
            &[("category", ""), ("back", &h("/transactions"))],
        )
        .await;
    assert_eq!(row.status, StatusCode::OK);
    assert!(
        row.body.starts_with(&format!("<tr id=\"t{txn}\"")),
        "{}",
        row.body
    );
    assert!(!row.body.contains("<html"));

    let csv = app.get(&h("/transactions.csv")).await;
    assert!(csv.body.starts_with("Date,Account,Description"));
}
