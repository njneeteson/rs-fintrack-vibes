//! The list of households (their data lives in household database folders).

use super::*;

impl AuthDb {
    pub fn households(&self) -> anyhow::Result<Vec<Household>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, slug, name, created_at FROM households ORDER BY name COLLATE NOCASE",
        )?;
        Ok(stmt
            .query_map([], household_from_row)?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn household_by_slug(&self, slug: &str) -> anyhow::Result<Option<Household>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT id, slug, name, created_at FROM households WHERE slug=?1",
                [slug],
                household_from_row,
            )
            .optional()?)
    }

    pub fn household(&self, id: i64) -> anyhow::Result<Option<Household>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT id, slug, name, created_at FROM households WHERE id=?1",
                [id],
                household_from_row,
            )
            .optional()?)
    }

    pub fn add_household(&self, slug: &str, name: &str) -> anyhow::Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO households (slug, name, created_at) VALUES (?1, ?2, ?3)",
            params![slug, name.trim(), now_text()],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn rename_household(&self, id: i64, name: &str) -> anyhow::Result<()> {
        self.conn().execute(
            "UPDATE households SET name=?1 WHERE id=?2",
            params![name.trim(), id],
        )?;
        Ok(())
    }

    pub fn remove_household(&self, id: i64) -> anyhow::Result<()> {
        self.conn()
            .execute("DELETE FROM households WHERE id=?1", [id])?;
        Ok(())
    }
}
