use super::Database;
use super::sql::get_enum;
use crate::categorize::{MatchInput, RuleEngine};
use crate::model::{Money, Rule};
use anyhow::ensure;
use rusqlite::params;

impl Database {
    pub fn rules(&self) -> anyhow::Result<Vec<Rule>> {
        self.query_all(
            "SELECT id, field, match_kind, pattern, direction, min_amount, max_amount, \
                account_id, category_id, priority, enabled
             FROM rules ORDER BY priority, id",
            [],
            |r| {
                Ok(Rule {
                    id: r.get(0)?,
                    field: get_enum(r, 1)?,
                    match_kind: get_enum(r, 2)?,
                    pattern: r.get(3)?,
                    direction: get_enum(r, 4)?,
                    min_amount: r.get(5)?,
                    max_amount: r.get(6)?,
                    account_id: r.get(7)?,
                    category_id: r.get(8)?,
                    priority: r.get(9)?,
                    enabled: r.get(10)?,
                })
            },
        )
    }

    /// Insert (`id == 0`) or update a rule; returns its id.
    /// Call [`Database::apply_rules`] afterwards.
    pub fn save_rule(&self, r: &Rule) -> anyhow::Result<i64> {
        ensure!(!r.pattern.trim().is_empty(), "rule pattern cannot be empty");
        ensure!(r.category_id != 0, "choose a category for the rule");
        crate::categorize::CompiledRule::compile(r).map_err(anyhow::Error::msg)?;
        let values = params![
            r.field.key(),
            r.match_kind.key(),
            r.pattern.trim(),
            r.direction.key(),
            r.min_amount,
            r.max_amount,
            r.account_id,
            r.category_id,
            r.priority,
            r.enabled,
            r.id
        ];
        if r.id == 0 {
            self.conn.execute(
                "INSERT INTO rules (field, match_kind, pattern, direction, min_amount, \
                    max_amount, account_id, category_id, priority, enabled)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                &values[..10],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE rules SET field=?1, match_kind=?2, pattern=?3, direction=?4, \
                    min_amount=?5, max_amount=?6,
                 account_id=?7, category_id=?8, priority=?9, enabled=?10 WHERE id=?11",
                values,
            )?;
            Ok(r.id)
        }
    }

    pub fn delete_rule(&self, id: i64) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM rules WHERE id=?1", [id])?;
        Ok(())
    }

    /// Re-run all rules over every transaction, updating automatic categories.
    /// Manual overrides are stored separately and are never changed. Returns
    /// the number of transactions whose automatic category changed.
    pub fn apply_rules(&self) -> anyhow::Result<usize> {
        let engine = RuleEngine::new(&self.rules()?);
        type Row = (i64, i64, Money, String, String, Option<i64>, Option<i64>);
        let rows: Vec<Row> = self.query_all(
            "SELECT id, account_id, amount, description, memo, auto_category_id, rule_id FROM \
                transactions",
            [],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )?;
        let tx = self.conn.unchecked_transaction()?;
        let mut changed = 0;
        {
            let mut stmt =
                tx.prepare("UPDATE transactions SET auto_category_id=?1, rule_id=?2 WHERE id=?3")?;
            for (id, account_id, amount, description, memo, old_cat, old_rule) in rows {
                let input = MatchInput {
                    account_id,
                    amount,
                    description: &description,
                    memo: &memo,
                };
                let (cat, rule) = engine.categorize(input);
                if (cat, rule) != (old_cat, old_rule) {
                    stmt.execute(params![cat, rule, id])?;
                    changed += 1;
                }
            }
        }
        tx.commit()?;
        Ok(changed)
    }
}
