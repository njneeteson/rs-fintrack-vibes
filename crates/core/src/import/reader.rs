//! Reading raw CSV files into a grid of trimmed strings.

use anyhow::{Context, bail};
use std::path::Path;

/// A CSV file as a list of rows, before any interpretation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RawCsv {
    pub file_name: String,
    pub rows: Vec<Vec<String>>,
}

impl RawCsv {
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self::from_bytes(name, &bytes)
    }

    /// Parse CSV bytes. Handles a UTF-8 BOM, Latin-1/Windows-1252 text,
    /// ragged rows and preamble lines; blank rows are dropped.
    pub fn from_bytes(file_name: impl Into<String>, bytes: &[u8]) -> anyhow::Result<Self> {
        let text = decode(bytes);
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(text.as_bytes());
        let mut rows = Vec::new();
        for record in reader.records() {
            let record = record.context("malformed CSV")?;
            let row: Vec<String> = record.iter().map(clean_cell).collect();
            if row.iter().any(|c| !c.is_empty()) {
                rows.push(row);
            }
        }
        if rows.is_empty() {
            bail!("the file contains no rows");
        }
        Ok(Self {
            file_name: file_name.into(),
            rows,
        })
    }

    pub fn max_columns(&self) -> usize {
        self.rows.iter().map(Vec::len).max().unwrap_or(0)
    }
}

fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        // Latin-1 fallback: every byte maps to the same code point.
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

/// Trim whitespace and a wrapping pair of apostrophes (BMO wraps card numbers in them).
fn clean_cell(cell: &str) -> String {
    let t = cell.trim();
    let t = if t.len() >= 2 && t.starts_with('\'') && t.ends_with('\'') {
        &t[1..t.len() - 1]
    } else {
        t
    };
    t.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ragged_csv_with_bom_and_preamble() {
        let data =
            b"\xEF\xBB\xBFFollowing data is valid as of 20260101\n\n a , b ,c\n'123',\"x, y\",3\n";
        let raw = RawCsv::from_bytes("t.csv", data).unwrap();
        assert_eq!(raw.rows.len(), 3);
        assert_eq!(raw.rows[1], vec!["a", "b", "c"]);
        assert_eq!(raw.rows[2], vec!["123", "x, y", "3"]);
        assert_eq!(raw.max_columns(), 3);
    }

    #[test]
    fn latin1_fallback() {
        let raw = RawCsv::from_bytes("t.csv", b"caf\xe9,1\n").unwrap();
        assert_eq!(raw.rows[0][0], "café");
    }
}
