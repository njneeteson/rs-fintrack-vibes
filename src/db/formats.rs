use super::Database;
use crate::model::CsvFormat;
use anyhow::ensure;
use rusqlite::params;

impl Database {
    pub fn formats(&self) -> anyhow::Result<Vec<CsvFormat>> {
        let rows: Vec<(i64, String, String, bool)> =
            self.query_all("SELECT id, name, spec, builtin FROM csv_formats ORDER BY builtin, name COLLATE NOCASE", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })?;
        rows.into_iter()
            .map(|(id, name, spec, builtin)| {
                let spec = serde_json::from_str(&spec).map_err(|e| anyhow::anyhow!("format '{name}' is corrupt: {e}"))?;
                Ok(CsvFormat { id, name, spec, builtin })
            })
            .collect()
    }

    /// Insert (`id == 0`) or update a format; returns its id.
    pub fn save_format(&self, f: &CsvFormat) -> anyhow::Result<i64> {
        ensure!(!f.name.trim().is_empty(), "format name cannot be empty");
        let spec = serde_json::to_string(&f.spec)?;
        let result = if f.id == 0 {
            self.conn
                .execute(
                    "INSERT INTO csv_formats (name, spec, builtin) VALUES (?1, ?2, ?3)",
                    params![f.name.trim(), spec, f.builtin],
                )
                .map(|_| self.conn.last_insert_rowid())
        } else {
            self.conn
                .execute("UPDATE csv_formats SET name=?1, spec=?2 WHERE id=?3", params![f.name.trim(), spec, f.id])
                .map(|_| f.id)
        };
        result.map_err(|e| anyhow::anyhow!("could not save format '{}': {e}", f.name))
    }

    pub fn delete_format(&self, id: i64) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM csv_formats WHERE id=?1", [id])?;
        Ok(())
    }
}
