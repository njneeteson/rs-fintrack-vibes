//! Sign-in, sessions, permissions and cross-site protection.

mod common;

use axum::http::StatusCode;
use common::{PASSWORD, TestApp, url};
use fintrack_server::auth::store::{Perm, Role, UserFields};

#[tokio::test]
async fn wrong_password_fails_and_is_throttled() {
    let mut app = TestApp::new();
    app.create_user("alice", Role::Admin);
    for _ in 0..5 {
        let r = app
            .post(
                &url("/login"),
                &[("username", "alice"), ("password", "nope")],
            )
            .await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);
        assert!(r.body.contains("Wrong username or password."));
    }
    // Throttled now, even with the right password.
    let r = app
        .post(
            &url("/login"),
            &[("username", "alice"), ("password", PASSWORD)],
        )
        .await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    // Unknown users get the same message.
    let r = app
        .post(&url("/login"), &[("username", "nobody"), ("password", "x")])
        .await;
    assert!(
        r.body.contains("Wrong username or password.") || r.status == StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn login_redirects_only_inside_the_app() {
    let mut app = TestApp::new();
    app.create_user("alice", Role::Admin);
    let next = [
        ("username", "alice"),
        ("password", PASSWORD),
        ("next", "https://evil.example/"),
    ];
    assert_eq!(
        app.post(&url("/login"), &next).await.redirect(),
        "/finances/"
    );
    let mut b = app.client();
    let next = [
        ("username", "alice"),
        ("password", PASSWORD),
        ("next", "/finances/account"),
    ];
    assert_eq!(
        b.post(&url("/login"), &next).await.redirect(),
        "/finances/account"
    );
}

#[tokio::test]
async fn logout_ends_the_session() {
    let mut app = TestApp::new();
    app.with_admin_and_household().await;
    assert_eq!(app.get(&url("/account")).await.status, StatusCode::OK);
    app.post(&url("/logout"), &[]).await;
    assert!(
        app.get(&url("/account"))
            .await
            .redirect()
            .starts_with("/finances/login")
    );
}

#[tokio::test]
async fn cross_site_posts_are_blocked() {
    let mut app = TestApp::new();
    let slug = app.with_admin_and_household().await;
    let path = url(&format!("/h/{slug}/categorize/apply"));
    assert_eq!(
        app.post_from(&path, &[], "cross-site").await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.post_from(&path, &[], "same-site").await.status,
        StatusCode::FORBIDDEN
    );
    assert!(app.post(&path, &[]).await.status.is_redirection());
}

#[tokio::test]
async fn household_access_follows_permissions() {
    let mut admin = TestApp::new();
    let slug = admin.with_admin_and_household().await;
    let other = admin
        .st
        .households
        .create_empty(&admin.st.auth, "Cottage")
        .unwrap();
    let viewer_id = admin.create_user("viewer", Role::User);
    let editor_id = admin.create_user("editor", Role::User);
    let home = admin.st.auth.household_by_slug(&slug).unwrap().unwrap();
    admin
        .st
        .auth
        .set_membership(viewer_id, home.id, Some(Perm::View))
        .unwrap();
    admin
        .st
        .auth
        .set_membership(editor_id, home.id, Some(Perm::Edit))
        .unwrap();

    let mut viewer = admin.client();
    viewer.login("viewer").await;
    assert_eq!(
        viewer
            .get(&url(&format!("/h/{slug}/transactions")))
            .await
            .status,
        StatusCode::OK
    );
    // A household they aren't a member of doesn't exist as far as they can tell.
    assert_eq!(
        viewer.get(&url(&format!("/h/{}", other.slug))).await.status,
        StatusCode::NOT_FOUND
    );
    // View-only: no changes, and edit-only pages are refused.
    let apply = url(&format!("/h/{slug}/categorize/apply"));
    assert_eq!(viewer.post(&apply, &[]).await.status, StatusCode::FORBIDDEN);
    assert_eq!(
        viewer.get(&url(&format!("/h/{slug}/import"))).await.status,
        StatusCode::FORBIDDEN
    );
    let page = viewer
        .get(&url(&format!("/h/{slug}/transactions")))
        .await
        .body;
    assert!(
        !page.contains("Import CSV"),
        "view-only users shouldn't see the import link"
    );
    // Not an admin.
    assert_eq!(
        viewer.get(&url("/admin/users")).await.status,
        StatusCode::FORBIDDEN
    );

    let mut editor = admin.client();
    editor.login("editor").await;
    assert!(editor.post(&apply, &[]).await.status.is_redirection());

    // Revoking access takes effect immediately.
    admin
        .st
        .auth
        .set_membership(viewer_id, home.id, None)
        .unwrap();
    assert_eq!(
        viewer.get(&url(&format!("/h/{slug}"))).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn admins_manage_users_but_not_the_last_admin() {
    let mut app = TestApp::new();
    app.with_admin_and_household().await;
    let r = app
        .post(
            &url("/admin/users"),
            &[
                ("username", "kid"),
                ("display_name", "Kid"),
                ("role", "user"),
            ],
        )
        .await;
    assert!(r.status.is_redirection());
    let kid = app.st.auth.credentials("kid").unwrap().unwrap().0;
    assert!(kid.must_change_password);

    let admin = app.st.auth.credentials("admin").unwrap().unwrap().0;
    // Can't delete yourself, and the last admin can't be demoted.
    assert_eq!(
        app.post(&url(&format!("/admin/users/{}/delete", admin.id)), &[])
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert!(
        app.st
            .auth
            .update_user(
                admin.id,
                &UserFields {
                    username: "admin".into(),
                    role: Some(Role::User),
                    ..Default::default()
                }
            )
            .is_err()
    );

    // Access matrix.
    let home = app.st.auth.households().unwrap()[0].clone();
    let field = format!("p_{}_{}", kid.id, home.id);
    app.post(&url("/admin/access"), &[(field.as_str(), "view")])
        .await;
    assert_eq!(
        app.st.auth.memberships().unwrap(),
        vec![(kid.id, home.id, Perm::View)]
    );
}

#[tokio::test]
async fn disabled_users_are_signed_out() {
    let mut app = TestApp::new();
    app.with_admin_and_household().await;
    let id = app.create_user("kid", Role::User);
    let mut kid = app.client();
    kid.login("kid").await;
    assert_eq!(kid.get(&url("/account")).await.status, StatusCode::OK);
    let r = app
        .post(
            &url(&format!("/admin/users/{id}")),
            &[("username", "kid"), ("role", "user"), ("disabled", "1")],
        )
        .await;
    assert!(r.status.is_redirection());
    assert!(
        kid.get(&url("/account"))
            .await
            .redirect()
            .starts_with("/finances/login")
    );
    let r = kid
        .post(
            &url("/login"),
            &[("username", "kid"), ("password", PASSWORD)],
        )
        .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn temporary_passwords_must_be_changed() {
    let mut app = TestApp::new();
    app.with_admin_and_household().await;
    let id = app.create_user("kid", Role::User);
    app.post(&url(&format!("/admin/users/{id}/reset-password")), &[])
        .await;
    let temp = "temp-pass-123";
    let hash = fintrack_server::auth::password::hash(temp).unwrap();
    app.st.auth.set_password(id, &hash, true).unwrap();

    let mut kid = app.client();
    let r = kid
        .post(&url("/login"), &[("username", "kid"), ("password", temp)])
        .await;
    assert!(r.status.is_redirection());
    assert_eq!(
        kid.get(&url("/households")).await.redirect(),
        "/finances/account/password"
    );
    let form = [
        ("current", temp),
        ("password", "my own password"),
        ("confirm", "my own password"),
    ];
    assert!(
        kid.post(&url("/account/password"), &form)
            .await
            .status
            .is_redirection()
    );
    assert_eq!(kid.get(&url("/households")).await.status, StatusCode::OK);
}

#[tokio::test]
async fn first_admin_via_setup_token() {
    let mut app = TestApp::new();
    let token = app
        .st
        .setup_token()
        .expect("no admin yet, so a setup token exists");
    assert_eq!(
        app.get(&url("/setup?token=wrong")).await.status,
        StatusCode::FORBIDDEN
    );
    let form = [
        ("token", token.as_str()),
        ("username", "root"),
        ("display_name", ""),
        ("password", PASSWORD),
        ("confirm", PASSWORD),
    ];
    let mut forged = form;
    forged[0].1 = "wrong";
    assert!(
        app.post(&url("/setup"), &forged)
            .await
            .redirect()
            .ends_with("/login")
    );
    assert_eq!(app.st.auth.count_admins().unwrap(), 0);
    assert!(
        app.post(&url("/setup"), &form)
            .await
            .status
            .is_redirection()
    );
    assert_eq!(app.st.auth.count_admins().unwrap(), 1);
    assert!(app.st.setup_token().is_none());
    // The link is single-use.
    assert!(
        app.get(&url(&format!("/setup?token={token}")))
            .await
            .redirect()
            .ends_with("/login")
    );
}
