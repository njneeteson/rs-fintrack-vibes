//! The import wizard end to end, including duplicate detection and column mapping.

mod common;

use axum::http::StatusCode;
use common::{TestApp, url};

const SCOTIA: &[u8] =
    b"Date,Description,Amount\n2026-03-01,COFFEE SHOP,-4.50\n2026-03-02,PAYROLL,2000.00\n";

#[tokio::test]
async fn upload_review_commit_and_dedupe() {
    let mut app = TestApp::new();
    app.create_user("admin", fintrack_server::auth::store::Role::Admin);
    app.login("admin").await;
    let hh = app
        .st
        .households
        .create_empty(&app.st.auth, "Home")
        .unwrap();
    let h = |p: &str| url(&format!("/h/{}{p}", hh.slug));

    // Unknown format → mapping page.
    let r = app
        .upload(&h("/import"), &[], ("files", "mybank.csv", SCOTIA))
        .await;
    let mapping = r.redirect().to_string();
    assert!(mapping.ends_with("/mapping"), "{mapping}");
    let page = app.get(&mapping).await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(
        page.body.contains("2 of about 2 rows read"),
        "guess should read both rows"
    );

    // Save the guessed mapping as a format.
    let form = [
        ("edited", "1"),
        ("name", "My bank"),
        ("header_row", "0"),
        ("date_column", "0"),
        ("description_column", "1"),
        ("amount_mode", "single"),
        ("amount_column", "2"),
        ("action", "save"),
    ];
    let review = app.post(&mapping, &form).await.redirect().to_string();
    let id = review.rsplit('/').next().unwrap().to_string();

    // Create an account for the rows, then commit.
    let new_account = format!("{review}/groups/0/new-account");
    app.post(&new_account, &[("name", "Chequing"), ("kind", "chequing")])
        .await;
    let page = app.get(&review).await;
    assert!(
        page.body.contains("Import 2 new transactions"),
        "{}",
        page.body
    );
    app.post(&format!("{review}/commit"), &[]).await;
    let done = app.get(&review).await;
    assert!(done.body.contains("Imported 2 new transactions"));
    app.post(&format!("{review}/cancel"), &[]).await;
    assert_eq!(
        app.get(&review).await.status,
        StatusCode::NOT_FOUND,
        "pending import {id} should be gone"
    );

    // The same file again: format and account are recognised; both rows are duplicates.
    let review2 = app
        .upload(&h("/import"), &[], ("files", "mybank.csv", SCOTIA))
        .await
        .redirect()
        .to_string();
    assert!(!review2.ends_with("/mapping"));
    let page = app.get(&review2).await;
    assert!(page.body.contains("(detected automatically)"));
    assert!(page.body.contains("Nothing new"), "{}", page.body);

    let db = app.st.households.get(&hh.slug).unwrap();
    assert_eq!(db.lock().unwrap().transactions().unwrap().len(), 2);
    assert_eq!(db.lock().unwrap().imports().unwrap().len(), 1);
}

#[tokio::test]
async fn pending_imports_are_private_to_their_user() {
    let mut app = TestApp::new();
    let slug = app.with_admin_and_household().await;
    let review = app
        .upload(
            &url(&format!("/h/{slug}/import")),
            &[],
            ("files", "a.csv", SCOTIA),
        )
        .await;
    let path = review.redirect().to_string();
    app.create_user("other", fintrack_server::auth::store::Role::Admin);
    let mut other = app.client();
    other.login("other").await;
    assert_eq!(other.get(&path).await.status, StatusCode::NOT_FOUND);
}
