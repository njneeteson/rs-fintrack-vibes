use super::Database;
use crate::import::ImportGroup;
use crate::import::dedup::{DedupStatus, FingerprintStats, classify};
use rusqlite::{Connection, params};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct ImportRecord {
    pub id: i64,
    pub file_name: String,
    pub archived_path: String,
    pub format_name: String,
    pub imported_at: String,
    pub rows_total: i64,
    pub rows_inserted: i64,
    pub rows_duplicate: i64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportSummary {
    pub import_id: i64,
    pub inserted: usize,
    pub duplicates: usize,
}

fn fingerprint_stats(
    conn: &Connection,
    account_id: i64,
    fps: &[String],
) -> rusqlite::Result<HashMap<String, FingerprintStats>> {
    let mut stmt = conn.prepare_cached(
        "SELECT COUNT(*), IFNULL(MAX(seq), 0) FROM transactions WHERE \
            account_id=?1 AND fingerprint=?2",
    )?;
    let mut out = HashMap::new();
    for fp in fps {
        if out.contains_key(fp) {
            continue;
        }
        let (count, max_seq): (i64, i64) =
            stmt.query_row(params![account_id, fp], |r| Ok((r.get(0)?, r.get(1)?)))?;
        out.insert(
            fp.clone(),
            FingerprintStats {
                count: count as usize,
                max_seq,
            },
        );
    }
    Ok(out)
}

impl Database {
    /// Duplicate status of each row in `group` if it were imported into `account_id`.
    pub fn preview_duplicates(
        &self,
        account_id: i64,
        group: &ImportGroup,
    ) -> anyhow::Result<Vec<DedupStatus>> {
        let fps = group.fingerprints();
        Ok(classify(
            &fps,
            &fingerprint_stats(&self.conn, account_id, &fps)?,
        ))
    }

    /// Insert all non-duplicate rows of groups that have an account, in one transaction.
    pub fn commit_import(
        &self,
        file_name: &str,
        archived_path: &str,
        format_name: &str,
        groups: &[ImportGroup],
    ) -> anyhow::Result<ImportSummary> {
        let tx = self.conn.unchecked_transaction()?;
        let total: usize = groups
            .iter()
            .filter(|g| g.account_id.is_some())
            .map(|g| g.rows.len())
            .sum();
        tx.execute(
            "INSERT INTO imports (file_name, archived_path, format_name, imported_at, rows_total, \
                rows_inserted, rows_duplicate)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0)",
            params![
                file_name,
                archived_path,
                format_name,
                chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                total
            ],
        )?;
        let mut summary = ImportSummary {
            import_id: tx.last_insert_rowid(),
            ..Default::default()
        };
        for g in groups {
            let Some(account_id) = g.account_id else {
                continue;
            };
            let fps = g.fingerprints();
            let statuses = classify(&fps, &fingerprint_stats(&tx, account_id, &fps)?);
            let mut insert = tx.prepare_cached(
                "INSERT INTO transactions (account_id, date, amount, description, memo, balance, \
                    fingerprint, seq, import_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;
            for ((row, fp), status) in g.rows.iter().zip(&fps).zip(statuses) {
                match status {
                    DedupStatus::Duplicate => summary.duplicates += 1,
                    DedupStatus::New { seq } => {
                        insert.execute(params![
                            account_id,
                            row.date,
                            row.amount,
                            row.description,
                            row.memo,
                            row.balance,
                            fp,
                            seq,
                            summary.import_id
                        ])?;
                        summary.inserted += 1;
                    }
                }
            }
        }
        tx.execute(
            "UPDATE imports SET rows_inserted=?1, rows_duplicate=?2 WHERE id=?3",
            params![summary.inserted, summary.duplicates, summary.import_id],
        )?;
        tx.commit()?;
        Ok(summary)
    }

    pub fn imports(&self) -> anyhow::Result<Vec<ImportRecord>> {
        self.query_all(
            "SELECT id, file_name, archived_path, format_name, imported_at, rows_total, \
                rows_inserted, rows_duplicate
             FROM imports ORDER BY id DESC",
            [],
            |r| {
                Ok(ImportRecord {
                    id: r.get(0)?,
                    file_name: r.get(1)?,
                    archived_path: r.get(2)?,
                    format_name: r.get(3)?,
                    imported_at: r.get(4)?,
                    rows_total: r.get(5)?,
                    rows_inserted: r.get(6)?,
                    rows_duplicate: r.get(7)?,
                })
            },
        )
    }

    /// Remove an import and every transaction it added.
    pub fn undo_import(&self, id: i64) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM imports WHERE id=?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::tests::temp_db;
    use crate::import::ImportGroup;
    use crate::import::parse::ParsedRow;
    use crate::model::{Account, Money, Rule};
    use chrono::NaiveDate;

    fn row(day: u32, cents: i64, desc: &str) -> ParsedRow {
        ParsedRow {
            line: 1,
            date: NaiveDate::from_ymd_opt(2026, 1, day).unwrap(),
            amount: Money(cents),
            description: desc.into(),
            memo: String::new(),
            balance: None,
            account_number: None,
        }
    }

    #[test]
    fn overlapping_imports_are_deduplicated_and_categorized() {
        let (_d, db) = temp_db();
        let acct = db
            .save_account(&Account {
                name: "Chq".into(),
                ..Default::default()
            })
            .unwrap();
        let first = ImportGroup {
            account_number: None,
            rows: vec![
                row(1, -450, "Coffee"),
                row(1, -450, "Coffee"),
                row(2, -9000, "SAVE ON FOODS #1"),
            ],
            account_id: Some(acct),
        };
        let s1 = db.commit_import("a.csv", "", "test", &[first]).unwrap();
        assert_eq!((s1.inserted, s1.duplicates), (3, 0));

        let second = ImportGroup {
            account_number: None,
            rows: vec![row(2, -9000, "save on foods #1"), row(3, -1200, "Lunch")],
            account_id: Some(acct),
        };
        let preview = db.preview_duplicates(acct, &second).unwrap();
        assert!(preview[0].is_duplicate() && !preview[1].is_duplicate());
        let s2 = db.commit_import("b.csv", "", "test", &[second]).unwrap();
        assert_eq!((s2.inserted, s2.duplicates), (1, 1));
        assert_eq!(db.transactions().unwrap().len(), 4);

        let groceries = crate::model::CategoryTree::new(db.categories().unwrap())
            .find("Groceries", None)
            .unwrap();
        db.save_rule(&Rule {
            pattern: "save on foods".into(),
            category_id: groceries,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(db.apply_rules().unwrap(), 1);
        assert_eq!(db.apply_rules().unwrap(), 0);
        let t = db.transactions().unwrap();
        let sof = t
            .iter()
            .find(|t| t.description.starts_with("SAVE"))
            .unwrap();
        assert_eq!(sof.category_id(), Some(groceries));

        db.set_manual_category(&[sof.id], None).unwrap();
        db.undo_import(s1.import_id).unwrap();
        assert_eq!(db.transactions().unwrap().len(), 1);
        assert_eq!(db.imports().unwrap().len(), 1);
    }
}
