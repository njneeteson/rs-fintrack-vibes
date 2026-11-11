//! Date ranges and the presets offered in the UI.

use super::period::{Granularity, Period};
use chrono::{Datelike, Months, NaiveDate};

/// An inclusive date range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    pub fn new(start: NaiveDate, end: NaiveDate) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    pub fn contains(&self, d: NaiveDate) -> bool {
        d >= self.start && d <= self.end
    }

    pub fn label(&self) -> String {
        format!(
            "{} – {}",
            self.start.format("%b %d, %Y"),
            self.end.format("%b %d, %Y")
        )
    }
}

str_enum! {
    pub enum RangePreset {
        ThisMonth => ("this_month", "This month"),
        LastMonth => ("last_month", "Last month"),
        Last3Months => ("last_3m", "Last 3 months"),
        Last6Months => ("last_6m", "Last 6 months"),
        Last12Months => ("last_12m", "Last 12 months"),
        YearToDate => ("ytd", "Year to date"),
        LastYear => ("last_year", "Last calendar year"),
        AllTime => ("all", "All time"),
        Custom => ("custom", "Custom range"),
    }
}

impl RangePreset {
    /// Resolve to concrete dates. "Last N months" means the current month plus
    /// the N-1 before it. `data_bounds` is used for "All time".
    pub fn resolve(
        self,
        today: NaiveDate,
        data_bounds: Option<DateRange>,
        custom: DateRange,
    ) -> DateRange {
        let month = Period::containing(today, Granularity::Month).start;
        let back = |n: u32| DateRange::new(month - Months::new(n - 1), today);
        match self {
            Self::ThisMonth => DateRange::new(month, today),
            Self::LastMonth => {
                let p = Period::containing(today, Granularity::Month).prev();
                DateRange::new(p.start, p.last_day())
            }
            Self::Last3Months => back(3),
            Self::Last6Months => back(6),
            Self::Last12Months => back(12),
            Self::YearToDate => {
                DateRange::new(NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap(), today)
            }
            Self::LastYear => DateRange::new(
                NaiveDate::from_ymd_opt(today.year() - 1, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(today.year() - 1, 12, 31).unwrap(),
            ),
            Self::AllTime => data_bounds.unwrap_or_else(|| DateRange::new(month, today)),
            Self::Custom => custom,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let c = DateRange::new(today, today);
        let r = RangePreset::Last3Months.resolve(today, None, c);
        assert_eq!(r.start, NaiveDate::from_ymd_opt(2026, 9, 1).unwrap());
        let r = RangePreset::LastMonth.resolve(today, None, c);
        assert_eq!(r.end, NaiveDate::from_ymd_opt(2026, 10, 31).unwrap());
        assert!(r.contains(NaiveDate::from_ymd_opt(2026, 10, 15).unwrap()));
    }
}
