use super::Database;
use super::sql::get_enum;
use crate::model::{Category, CategoryKind};
use anyhow::{bail, ensure};
use rusqlite::{OptionalExtension, params};

impl Database {
    pub fn categories(&self) -> anyhow::Result<Vec<Category>> {
        self.query_all(
            "SELECT id, name, parent_id, kind FROM categories",
            [],
            |r| {
                Ok(Category {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    parent_id: r.get(2)?,
                    kind: get_enum(r, 3)?,
                })
            },
        )
    }

    fn parent_of(&self, id: i64) -> anyhow::Result<Option<Option<i64>>> {
        Ok(self
            .conn
            .query_row("SELECT parent_id FROM categories WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?)
    }

    fn kind_of(&self, id: i64) -> anyhow::Result<CategoryKind> {
        Ok(self
            .conn
            .query_row("SELECT kind FROM categories WHERE id=?1", [id], |r| {
                get_enum(r, 0)
            })?)
    }

    /// Ensure `parent` is a top-level category (the hierarchy has two levels).
    fn check_parent(&self, parent: Option<i64>, child: Option<i64>) -> anyhow::Result<()> {
        if let Some(p) = parent {
            match self.parent_of(p)? {
                None => bail!("parent category no longer exists"),
                Some(Some(_)) => bail!("subcategories cannot have their own subcategories"),
                Some(None) => {}
            }
            ensure!(Some(p) != child, "a category cannot be its own parent");
            if let Some(c) = child {
                let n: i64 = self.conn.query_row(
                    "SELECT COUNT(*) FROM categories WHERE parent_id=?1",
                    [c],
                    |r| r.get(0),
                )?;
                ensure!(
                    n == 0,
                    "move or delete this category's subcategories before nesting it"
                );
            }
        }
        Ok(())
    }

    /// Add a category. Subcategories always take their parent's kind.
    pub fn add_category(
        &self,
        name: &str,
        parent_id: Option<i64>,
        kind: CategoryKind,
    ) -> anyhow::Result<i64> {
        let name = name.trim();
        ensure!(!name.is_empty(), "category name cannot be empty");
        self.check_parent(parent_id, None)?;
        let kind = match parent_id {
            Some(p) => self.kind_of(p)?,
            None => kind,
        };
        self.conn
            .execute(
                "INSERT INTO categories (name, parent_id, kind) VALUES (?1, ?2, ?3)",
                params![name, parent_id, kind.key()],
            )
            .map_err(|e| anyhow::anyhow!("could not add '{name}': {e}"))?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Rename / re-parent / change the kind of a category.
    pub fn update_category(
        &self,
        id: i64,
        name: &str,
        parent_id: Option<i64>,
        kind: CategoryKind,
    ) -> anyhow::Result<()> {
        let name = name.trim();
        ensure!(!name.is_empty(), "category name cannot be empty");
        self.check_parent(parent_id, Some(id))?;
        let kind = match parent_id {
            Some(p) => self.kind_of(p)?,
            None => kind,
        };
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE categories SET name=?1, parent_id=?2, kind=?3 WHERE id=?4",
            params![name, parent_id, kind.key(), id],
        )
        .map_err(|e| anyhow::anyhow!("could not rename to '{name}': {e}"))?;
        tx.execute(
            "UPDATE categories SET kind=?1 WHERE parent_id=?2",
            params![kind.key(), id],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Delete a category (and its subcategories). With `reassign_to`, its
    /// transactions, rules and budget are moved to that category first.
    pub fn delete_category(&self, id: i64, reassign_to: Option<i64>) -> anyhow::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        if let Some(target) = reassign_to {
            let affected = "(SELECT id FROM categories WHERE id=?1 OR parent_id=?1)";
            let target_is_deleted: bool = tx.query_row(
                &format!("SELECT ?2 IN {affected}"),
                params![id, target],
                |r| r.get(0),
            )?;
            ensure!(
                !target_is_deleted,
                "cannot reassign to the category being deleted"
            );
            tx.execute(
                &format!(
                    "UPDATE transactions SET manual_category_id=?2 WHERE manual_category_id \
                    IN {affected}"
                ),
                params![id, target],
            )?;
            tx.execute(
                &format!(
                    "UPDATE transactions SET auto_category_id=?2 WHERE auto_category_id IN \
                    {affected}"
                ),
                params![id, target],
            )?;
            tx.execute(
                &format!("UPDATE rules SET category_id=?2 WHERE category_id IN {affected}"),
                params![id, target],
            )?;
            tx.execute(
                &format!(
                    "UPDATE OR IGNORE budgets SET category_id=?2 WHERE category_id IN {affected}"
                ),
                params![id, target],
            )?;
        }
        tx.execute("DELETE FROM categories WHERE id=?1", [id])?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::tests::temp_db;
    use crate::model::{CategoryKind, CategoryTree};

    #[test]
    fn hierarchy_rules_and_reassign() {
        let (_d, db) = temp_db();
        let pets = db
            .add_category("Pets", None, CategoryKind::Expense)
            .unwrap();
        let food = db
            .add_category("Food", Some(pets), CategoryKind::Income)
            .unwrap();
        assert!(
            db.add_category("Deep", Some(food), CategoryKind::Expense)
                .is_err()
        );
        assert!(
            db.add_category("food", Some(pets), CategoryKind::Expense)
                .is_err(),
            "duplicate names rejected"
        );
        let tree = CategoryTree::new(db.categories().unwrap());
        assert_eq!(
            tree.kind(food),
            Some(CategoryKind::Expense),
            "children inherit kind"
        );

        db.update_category(pets, "Pets", None, CategoryKind::Transfer)
            .unwrap();
        let tree = CategoryTree::new(db.categories().unwrap());
        assert_eq!(tree.get(food).unwrap().kind, CategoryKind::Transfer);
        assert!(
            db.update_category(pets, "Pets", Some(food), CategoryKind::Expense)
                .is_err()
        );

        let groceries = tree.find("Groceries", None).unwrap();
        db.set_budget(food, crate::model::Money(100)).unwrap();
        assert!(db.delete_category(pets, Some(food)).is_err());
        db.delete_category(pets, Some(groceries)).unwrap();
        let tree = CategoryTree::new(db.categories().unwrap());
        assert!(tree.get(food).is_none());
        assert_eq!(db.budgets().unwrap()[0].category_id, groceries);
    }
}
