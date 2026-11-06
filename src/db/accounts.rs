use super::Database;
use super::sql::get_enum;
use crate::model::Account;
use rusqlite::{Row, params};

const COLS: &str = "id, name, institution, kind, account_number, filename_hint, format_id, opening_balance, archived";

fn map(r: &Row<'_>) -> rusqlite::Result<Account> {
    Ok(Account {
        id: r.get(0)?,
        name: r.get(1)?,
        institution: r.get(2)?,
        kind: get_enum(r, 3)?,
        account_number: r.get(4)?,
        filename_hint: r.get(5)?,
        format_id: r.get(6)?,
        opening_balance: r.get(7)?,
        archived: r.get(8)?,
    })
}

impl Database {
    pub fn accounts(&self) -> anyhow::Result<Vec<Account>> {
        self.query_all(&format!("SELECT {COLS} FROM accounts ORDER BY archived, name COLLATE NOCASE"), [], map)
    }

    /// Insert (`id == 0`) or update an account; returns its id.
    pub fn save_account(&self, a: &Account) -> anyhow::Result<i64> {
        anyhow::ensure!(!a.name.trim().is_empty(), "account name cannot be empty");
        let p = params![
            a.name.trim(),
            a.institution.trim(),
            a.kind.key(),
            a.account_number.trim(),
            a.filename_hint.trim(),
            a.format_id,
            a.opening_balance,
            a.archived,
            a.id
        ];
        if a.id == 0 {
            self.conn.execute(
                "INSERT INTO accounts (name, institution, kind, account_number, filename_hint, format_id, opening_balance, archived)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                &p[..8],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE accounts SET name=?1, institution=?2, kind=?3, account_number=?4, filename_hint=?5,
                 format_id=?6, opening_balance=?7, archived=?8 WHERE id=?9",
                p,
            )?;
            Ok(a.id)
        }
    }

    /// Delete an account and all its transactions.
    pub fn delete_account(&self, id: i64) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM accounts WHERE id=?1", [id])?;
        Ok(())
    }

    /// Remember identifiers seen during an import so the next import is detected automatically.
    pub fn learn_account_identity(&self, id: i64, account_number: Option<&str>, format_id: Option<i64>) -> anyhow::Result<()> {
        if let Some(n) = account_number.filter(|n| !n.trim().is_empty()) {
            self.conn
                .execute("UPDATE accounts SET account_number=?1 WHERE id=?2 AND account_number=''", params![n.trim(), id])?;
        }
        if let Some(f) = format_id {
            self.conn.execute("UPDATE accounts SET format_id=?1 WHERE id=?2 AND format_id IS NULL", params![f, id])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::tests::temp_db;
    use crate::model::{Account, AccountKind, Money};

    #[test]
    fn crud_and_learning() {
        let (_d, db) = temp_db();
        let id = db
            .save_account(&Account {
                name: "Visa".into(),
                kind: AccountKind::CreditCard,
                opening_balance: Money(-500),
                ..Default::default()
            })
            .unwrap();
        db.learn_account_identity(id, Some("1234"), Some(1)).unwrap();
        db.learn_account_identity(id, Some("9999"), Some(2)).unwrap();
        let mut a = db.accounts().unwrap().remove(0);
        assert_eq!((a.account_number.as_str(), a.format_id, a.opening_balance), ("1234", Some(1), Money(-500)));
        a.name = "Scotia Visa".into();
        db.save_account(&a).unwrap();
        assert_eq!(db.accounts().unwrap()[0].name, "Scotia Visa");
        assert!(db.save_account(&Account::default()).is_err());
        db.delete_account(id).unwrap();
        assert!(db.accounts().unwrap().is_empty());
    }
}
