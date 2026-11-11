//! An import in progress, held in memory between the wizard's steps.

use fintrack_core::db::{Database, ImportSummary};
use fintrack_core::import::detect::detect_format;
use fintrack_core::import::parse::{RowError, parse_all};
use fintrack_core::import::reader::RawCsv;
use fintrack_core::import::{ImportGroup, assign_accounts, group_by_account};
use fintrack_core::model::CsvFormat;

const UNKNOWN_FORMAT: &str = "This file's format isn't known yet. \
    Map its columns once and FinTrack will recognise it next time.";

/// An uploaded file.
pub struct Upload {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

/// The file read with a format, split by account.
pub struct Parsed {
    pub groups: Vec<ImportGroup>,
    pub errors: Vec<RowError>,
    pub skipped: usize,
}

#[derive(Clone)]
pub struct Done {
    pub summary: ImportSummary,
    pub categorized: usize,
    pub unassigned_rows: usize,
}

pub struct PendingImport {
    pub household_id: i64,
    pub user_id: i64,
    pub upload: Upload,
    pub raw: RawCsv,
    /// Files uploaded together with this one, imported next.
    pub queue: Vec<Upload>,
    /// The accepted format; `None` while the columns still need mapping.
    pub format: Option<CsvFormat>,
    /// Format to start the mapping form from (a detected format that didn't fit).
    pub mapping_base: Option<CsvFormat>,
    pub parsed: Option<Parsed>,
    pub done: Option<Done>,
}

/// Read `raw` with `format`. `Err` explains why the mapping needs attention.
pub fn parse(db: &Database, raw: &RawCsv, format: &CsvFormat) -> Result<Parsed, String> {
    let out = parse_all(raw, &format.spec).map_err(|e| format!("{}: {e}", format.name))?;
    if out.rows.is_empty() {
        return Err(format!(
            "No rows could be read with the format “{}”. Check the column mapping.",
            format.name
        ));
    }
    let mut groups = group_by_account(out.rows);
    let accounts = db.accounts().map_err(|e| format!("{e:#}"))?;
    assign_accounts(&mut groups, &accounts, &raw.file_name, Some(format.id));
    Ok(Parsed {
        groups,
        errors: out.errors,
        skipped: out.skipped,
    })
}

impl PendingImport {
    /// Read an upload and try to recognise its format. Returns a message when
    /// the file needs its columns mapped first.
    pub fn start(
        db: &Database,
        household_id: i64,
        user_id: i64,
        upload: Upload,
        queue: Vec<Upload>,
    ) -> anyhow::Result<(Self, Option<String>)> {
        let raw = RawCsv::from_bytes(upload.file_name.clone(), &upload.bytes)?;
        let mut p = Self {
            household_id,
            user_id,
            upload,
            raw,
            queue,
            format: None,
            mapping_base: None,
            parsed: None,
            done: None,
        };
        let message = p.detect(db)?;
        Ok((p, message))
    }

    fn detect(&mut self, db: &Database) -> anyhow::Result<Option<String>> {
        let formats = db.formats()?;
        let Some(d) = detect_format(&self.raw, &formats) else {
            return Ok(Some(UNKNOWN_FORMAT.into()));
        };
        Ok(self.use_format(db, formats[d.format_index].clone()).err())
    }

    /// Accept `format` if it reads the file; otherwise keep it as the mapping starting point.
    pub fn use_format(&mut self, db: &Database, format: CsvFormat) -> Result<(), String> {
        match parse(db, &self.raw, &format) {
            Ok(parsed) => {
                self.parsed = Some(parsed);
                self.format = Some(format);
                self.mapping_base = None;
                Ok(())
            }
            Err(msg) => {
                self.parsed = None;
                self.format = None;
                self.mapping_base = Some(format);
                Err(msg)
            }
        }
    }

    /// Start an import of the next queued file (taking the rest of the queue with it).
    pub fn next_file(&mut self, db: &Database) -> anyhow::Result<Option<(Self, Option<String>)>> {
        let mut queue = std::mem::take(&mut self.queue);
        if queue.is_empty() {
            return Ok(None);
        }
        let upload = queue.remove(0);
        Self::start(db, self.household_id, self.user_id, upload, queue).map(Some)
    }
}
