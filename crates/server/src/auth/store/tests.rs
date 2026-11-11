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
