//! Duplicate detection for overlapping CSV imports.
//!
//! Each transaction gets a *fingerprint* (date, amount, normalized description)
//! and a *sequence number* among identical fingerprints in its account. When a
//! file is imported, the k-th occurrence of a fingerprint in the file is a
//! duplicate if the account already holds at least k transactions with that
//! fingerprint. This multiset rule means:
//!
//! * re-importing an overlapping export skips the rows already stored, and
//! * two genuinely identical purchases on the same day (two coffees) are both
//!   kept, because they appear twice in the file.

use crate::model::{Money, normalize_text};
use chrono::NaiveDate;
use std::collections::HashMap;

pub fn fingerprint(date: NaiveDate, amount: Money, description: &str) -> String {
    format!("{date}|{}|{}", amount.cents(), normalize_text(description))
}

/// What the database already holds for one fingerprint in one account.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FingerprintStats {
    pub count: usize,
    pub max_seq: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DedupStatus {
    /// Insert with this sequence number.
    New {
        seq: i64,
    },
    Duplicate,
}

impl DedupStatus {
    pub fn is_duplicate(self) -> bool {
        self == DedupStatus::Duplicate
    }
}

/// Classify each incoming fingerprint (in file order) against existing stats.
pub fn classify(
    fingerprints: &[String],
    existing: &HashMap<String, FingerprintStats>,
) -> Vec<DedupStatus> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    fingerprints
        .iter()
        .map(|fp| {
            let ordinal = {
                let n = seen.entry(fp.as_str()).or_default();
                *n += 1;
                *n
            };
            let stats = existing.get(fp).copied().unwrap_or_default();
            if ordinal <= stats.count {
                DedupStatus::Duplicate
            } else {
                DedupStatus::New {
                    seq: stats.max_seq + (ordinal - stats.count) as i64,
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_and_legit_repeats() {
        let d = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let coffee = fingerprint(d, Money(-450), "Coffee  shop");
        let rent = fingerprint(d, Money(-200000), "RENT");
        assert_eq!(coffee, fingerprint(d, Money(-450), "COFFEE SHOP"));

        // Nothing stored yet: two identical coffees are both new.
        let st = classify(&[coffee.clone(), coffee.clone()], &HashMap::new());
        assert_eq!(
            st,
            vec![DedupStatus::New { seq: 1 }, DedupStatus::New { seq: 2 }]
        );

        // One coffee already stored; an overlapping file with two coffees + rent.
        let existing = HashMap::from([(
            coffee.clone(),
            FingerprintStats {
                count: 1,
                max_seq: 1,
            },
        )]);
        let st = classify(&[coffee.clone(), rent, coffee], &existing);
        assert_eq!(
            st,
            vec![
                DedupStatus::Duplicate,
                DedupStatus::New { seq: 1 },
                DedupStatus::New { seq: 2 }
            ]
        );
    }
}
