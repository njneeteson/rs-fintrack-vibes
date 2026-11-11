use super::*;

fn fields(name: &str, role: Role) -> UserFields {
    UserFields {
        username: name.into(),
        role: Some(role),
        ..Default::default()
    }
}

#[test]
fn last_admin_is_protected() {
    let db = AuthDb::in_memory().unwrap();
    let a = db
        .create_user(&fields("alice", Role::Admin), "h", false)
        .unwrap();
    assert!(db.delete_user(a).is_err());
    assert!(db.update_user(a, &fields("alice", Role::User)).is_err());
    assert!(
        db.update_user(
            a,
            &UserFields {
                disabled: true,
                ..fields("alice", Role::Admin)
            }
        )
        .is_err()
    );
    let b = db
        .create_user(&fields("bob", Role::Admin), "h", false)
        .unwrap();
    db.update_user(a, &fields("alice", Role::User)).unwrap();
    assert!(db.delete_user(b).is_err());
    db.delete_user(a).unwrap();
}

#[test]
fn usernames_are_unique_and_case_insensitive() {
    let db = AuthDb::in_memory().unwrap();
    db.create_user(&fields("Alice", Role::User), "h", false)
        .unwrap();
    assert!(
        db.create_user(&fields("alice", Role::User), "h", false)
            .is_err()
    );
    assert!(db.credentials("ALICE").unwrap().is_some());
    assert!(
        db.create_user(&fields("bad name", Role::User), "h", false)
            .is_err()
    );
}

#[test]
fn access_follows_memberships() {
    let db = AuthDb::in_memory().unwrap();
    let admin = db
        .create_user(&fields("root", Role::Admin), "h", false)
        .unwrap();
    let u = db
        .create_user(&fields("kid", Role::User), "h", false)
        .unwrap();
    let h1 = db.add_household("home", "Home").unwrap();
    let h2 = db.add_household("cottage", "Cottage").unwrap();
    let user = db.user(u).unwrap().unwrap();
    assert!(db.access_for(&user).unwrap().is_empty());
    db.set_membership(u, h1, Some(Perm::View)).unwrap();
    db.set_membership(u, h1, Some(Perm::Edit)).unwrap();
    let access = db.access_for(&user).unwrap();
    assert_eq!(access.len(), 1);
    assert_eq!(access[0].perm, Perm::Edit);
    db.set_membership(u, h1, None).unwrap();
    assert!(db.access_for(&user).unwrap().is_empty());
    let all = db.access_for(&db.user(admin).unwrap().unwrap()).unwrap();
    assert_eq!(all.len(), 2);
    db.set_membership(u, h2, Some(Perm::View)).unwrap();
    db.remove_household(h2).unwrap();
    assert!(db.memberships().unwrap().is_empty());
}

#[test]
fn sessions_expire_and_die_with_disable() {
    let db = AuthDb::in_memory().unwrap();
    let _admin = db
        .create_user(&fields("root", Role::Admin), "h", false)
        .unwrap();
    let u = db
        .create_user(&fields("kid", Role::User), "h", false)
        .unwrap();
    db.insert_session("t1", u, true, 2_000).unwrap();
    assert!(db.session_user("t1", 1_000).unwrap().is_some());
    assert!(db.session_user("t1", 2_000).unwrap().is_none());
    db.update_user(
        u,
        &UserFields {
            disabled: true,
            ..fields("kid", Role::User)
        },
    )
    .unwrap();
    assert_eq!(db.session_count(u).unwrap(), 0);
}

#[test]
fn guest_account_lifecycle() {
    let db = AuthDb::in_memory().unwrap();
    assert!(db.guest().unwrap().is_none());
    // A real person called "guest" keeps their name; the guest account gets another.
    db.create_user(&fields("guest", Role::User), "h", false)
        .unwrap();
    let g = db.set_guest(true, "Visitor").unwrap();
    assert!(g.guest && !g.disabled);
    assert_eq!(g.username, "guest-2");
    assert_eq!(g.name(), "Visitor");
    assert_eq!(db.enabled_guest().unwrap().map(|u| u.id), Some(g.id));
    // No password sign-in for the guest account.
    assert!(db.credentials("guest-2").unwrap().is_none());
    assert!(!db.credentials("guest").unwrap().unwrap().0.guest);
    // Turning it off keeps the account (and its access) but signs guests out.
    db.insert_session("t", g.id, false, i64::MAX).unwrap();
    let off = db.set_guest(false, "Visitor").unwrap();
    assert_eq!(off.id, g.id);
    assert!(off.disabled);
    assert!(db.enabled_guest().unwrap().is_none());
    assert_eq!(db.session_count(g.id).unwrap(), 0);
}

#[test]
fn old_databases_gain_the_guest_column() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, username TEXT NOT NULL UNIQUE COLLATE NOCASE,
         display_name TEXT NOT NULL DEFAULT '', password_hash TEXT NOT NULL, role TEXT NOT NULL,
         disabled INTEGER NOT NULL DEFAULT 0, must_change_password INTEGER NOT NULL DEFAULT 0,
         default_household INTEGER, theme TEXT NOT NULL DEFAULT 'system',
         created_at TEXT NOT NULL, last_login TEXT);
         INSERT INTO users (username, password_hash, role, created_at)
         VALUES ('old', 'h', 'admin', 'x');",
    )
    .unwrap();
    let db = AuthDb::init(conn).unwrap();
    let users = db.users().unwrap();
    assert_eq!(users.len(), 1);
    assert!(!users[0].guest);
}
