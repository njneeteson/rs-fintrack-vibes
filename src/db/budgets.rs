use super::Database;
use crate::model::{Budget, Money};
use rusqlite::params;

impl Database {
    pub fn budgets(&self) -> anyhow::Result<Vec<Budget>> {
        self.query_all("SELECT id, category_id, monthly_amount FROM budgets", [], |r| {
            Ok(Budget { id: r.get(0)?, category_id: r.get(1)?, monthly_amount: r.get(2)? })
        })
    }

    /// Create or update the monthly budget for a category.
    pub fn set_budget(&self, category_id: i64, monthly_amount: Money) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO budgets (category_id, monthly_amount) VALUES (?1, ?2)
             ON CONFLICT(category_id) DO UPDATE SET monthly_amount=excluded.monthly_amount",
            params![category_id, monthly_amount.abs()],
        )?;
        Ok(())
    }

    pub fn delete_budget(&self, id: i64) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM budgets WHERE id=?1", [id])?;
        Ok(())
    }
}
