//! Schema creation and migrations (tracked with `PRAGMA user_version`).

use rusqlite::Connection;

const V1: &str = r#"
CREATE TABLE categories (
    id        INTEGER PRIMARY KEY,
    name      TEXT NOT NULL,
    parent_id INTEGER REFERENCES categories(id) ON DELETE CASCADE,
    kind      TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_categories_name ON categories(IFNULL(parent_id, 0), name COLLATE NOCASE);

CREATE TABLE csv_formats (
    id      INTEGER PRIMARY KEY,
    name    TEXT NOT NULL UNIQUE,
    spec    TEXT NOT NULL,
    builtin INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE accounts (
    id              INTEGER PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,
    institution     TEXT NOT NULL DEFAULT '',
    kind            TEXT NOT NULL,
    account_number  TEXT NOT NULL DEFAULT '',
    filename_hint   TEXT NOT NULL DEFAULT '',
    format_id       INTEGER REFERENCES csv_formats(id) ON DELETE SET NULL,
    opening_balance INTEGER NOT NULL DEFAULT 0,
    archived        INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE imports (
    id             INTEGER PRIMARY KEY,
    file_name      TEXT NOT NULL,
    archived_path  TEXT NOT NULL DEFAULT '',
    format_name    TEXT NOT NULL DEFAULT '',
    imported_at    TEXT NOT NULL,
    rows_total     INTEGER NOT NULL,
    rows_inserted  INTEGER NOT NULL,
    rows_duplicate INTEGER NOT NULL
);

CREATE TABLE rules (
    id          INTEGER PRIMARY KEY,
    field       TEXT NOT NULL,
    match_kind  TEXT NOT NULL,
    pattern     TEXT NOT NULL,
    direction   TEXT NOT NULL,
    min_amount  INTEGER,
    max_amount  INTEGER,
    account_id  INTEGER REFERENCES accounts(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    priority    INTEGER NOT NULL DEFAULT 100,
    enabled     INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE transactions (
    id                 INTEGER PRIMARY KEY,
    account_id         INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    date               TEXT NOT NULL,
    amount             INTEGER NOT NULL,
    description        TEXT NOT NULL,
    memo               TEXT NOT NULL DEFAULT '',
    balance            INTEGER,
    fingerprint        TEXT NOT NULL,
    seq                INTEGER NOT NULL,
    import_id          INTEGER REFERENCES imports(id) ON DELETE CASCADE,
    auto_category_id   INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    rule_id            INTEGER REFERENCES rules(id) ON DELETE SET NULL,
    manual_category_id INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    note               TEXT NOT NULL DEFAULT '',
    UNIQUE (account_id, fingerprint, seq)
);
CREATE INDEX idx_transactions_date ON transactions(date);

CREATE TABLE budgets (
    id             INTEGER PRIMARY KEY,
    category_id    INTEGER NOT NULL UNIQUE REFERENCES categories(id) ON DELETE CASCADE,
    monthly_amount INTEGER NOT NULL
);

CREATE TABLE mortgages (
    id                  INTEGER PRIMARY KEY,
    name                TEXT NOT NULL,
    principal           INTEGER NOT NULL,
    annual_rate         REAL NOT NULL,
    amortization_months INTEGER NOT NULL,
    start_date          TEXT NOT NULL,
    frequency           TEXT NOT NULL,
    payment_override    INTEGER,
    term_months         INTEGER NOT NULL DEFAULT 60,
    notes               TEXT NOT NULL DEFAULT ''
);

CREATE TABLE mortgage_prepayments (
    id          INTEGER PRIMARY KEY,
    mortgage_id INTEGER NOT NULL REFERENCES mortgages(id) ON DELETE CASCADE,
    date        TEXT NOT NULL,
    amount      INTEGER NOT NULL,
    note        TEXT NOT NULL DEFAULT '',
    enabled     INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE mortgage_rate_changes (
    id             INTEGER PRIMARY KEY,
    mortgage_id    INTEGER NOT NULL REFERENCES mortgages(id) ON DELETE CASCADE,
    effective_date TEXT NOT NULL,
    annual_rate    REAL NOT NULL
);
"#;

/// Ordered migrations; index + 1 is the resulting schema version.
const MIGRATIONS: &[&str] = &[V1];

pub fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "foreign_keys", true)?;
    let version: usize = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i + 1)?;
        tx.commit()?;
    }
    Ok(())
}
