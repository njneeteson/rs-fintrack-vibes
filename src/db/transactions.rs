use super::Database;
use crate::model::Transaction;
use rusqlite::params;

impl Database {
    /// All transactions, newest first.
    pub fn transactions(&self) -> anyhow::Result<Vec<Transaction>> {
        self.query_all(
            "SELECT id, account_id, date, amount, description, memo, balance, auto_category_id, rule_id,
                    manual_category_id, note, import_id
             FROM transactions ORDER BY date DESC, id DESC",
            [],
            |r| {
                Ok(Transaction {
                    id: r.get(0)?,
                    account_id: r.get(1)?,
                    date: r.get(2)?,
                    amount: r.get(3)?,
                    description: r.get(4)?,
                    memo: r.get(5)?,
                    balance: r.get(6)?,
                    auto_category_id: r.get(7)?,
                    rule_id: r.get(8)?,
                    manual_category_id: r.get(9)?,
                    note: r.get(10)?,
                    import_id: r.get(11)?,
                })
            },
        )
    }

    /// Set (or with `None`, clear) the manual category override of several transactions.
    pub fn set_manual_category(&self, ids: &[i64], category_id: Option<i64>) -> anyhow::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare("UPDATE transactions SET manual_category_id=?1 WHERE id=?2")?;
            for id in ids {
                stmt.execute(params![category_id, id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn set_note(&self, id: i64, note: &str) -> anyhow::Result<()> {
        self.conn.execute("UPDATE transactions SET note=?1 WHERE id=?2", params![note, id])?;
        Ok(())
    }

    pub fn delete_transactions(&self, ids: &[i64]) -> anyhow::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare("DELETE FROM transactions WHERE id=?1")?;
            for id in ids {
                stmt.execute([id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}
