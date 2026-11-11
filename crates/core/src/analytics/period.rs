//! Calendar periods (weeks, months, quarters, years).

use chrono::{Datelike, Days, Months, NaiveDate};

str_enum! {
    pub enum Granularity {
        Week => ("week", "Weekly"),
        Month => ("month", "Monthly"),
        Quarter => ("quarter", "Quarterly"),
        Year => ("year", "Annual"),
    }
}

impl Granularity {
    /// Length in months (weeks are expressed as a fraction) — used to scale monthly budgets.
    pub fn months(self) -> f64 {
        match self {
            Self::Week => 12.0 / 52.0,
            Self::Month => 1.0,
            Self::Quarter => 3.0,
            Self::Year => 12.0,
        }
    }
}

/// A calendar period identified by its first day. Weeks start on Monday.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Period {
    pub start: NaiveDate,
    pub granularity: Granularity,
}

fn first_of_month(y: i32, m: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, 1).expect("valid month")
}

impl Period {
    pub fn containing(date: NaiveDate, granularity: Granularity) -> Self {
        let start = match granularity {
            Granularity::Week => date - Days::new(u64::from(date.weekday().num_days_from_monday())),
            Granularity::Month => first_of_month(date.year(), date.month()),
            Granularity::Quarter => first_of_month(date.year(), (date.month() - 1) / 3 * 3 + 1),
            Granularity::Year => first_of_month(date.year(), 1),
        };
        Self { start, granularity }
    }

    fn shift(self, forward: bool) -> Self {
        let start = match self.granularity {
            Granularity::Week if forward => self.start + Days::new(7),
            Granularity::Week => self.start - Days::new(7),
            g => {
                let m = Months::new(g.months() as u32);
                if forward {
                    self.start + m
                } else {
                    self.start - m
                }
            }
        };
        Self {
            start,
            granularity: self.granularity,
        }
    }

    pub fn next(self) -> Self {
        self.shift(true)
    }

    pub fn prev(self) -> Self {
        self.shift(false)
    }

    /// Last day inside the period.
    pub fn last_day(self) -> NaiveDate {
        self.next().start - Days::new(1)
    }

    pub fn contains(self, date: NaiveDate) -> bool {
        date >= self.start && date < self.next().start
    }

    /// Full label, e.g. `Week of 2026-02-02`, `Feb 2026`, `Q1 2026`, `2026`.
    pub fn label(self) -> String {
        match self.granularity {
            Granularity::Week => format!("Week of {}", self.start.format("%Y-%m-%d")),
            Granularity::Month => self.start.format("%b %Y").to_string(),
            Granularity::Quarter => format!(
                "Q{} {}",
                (self.start.month() - 1) / 3 + 1,
                self.start.year()
            ),
            Granularity::Year => self.start.year().to_string(),
        }
    }

    /// Compact label for chart axes.
    pub fn short_label(self) -> String {
        match self.granularity {
            Granularity::Week => self.start.format("%b %d").to_string(),
            Granularity::Month => self.start.format("%b %y").to_string(),
            Granularity::Quarter => format!(
                "Q{} {}",
                (self.start.month() - 1) / 3 + 1,
                self.start.format("%y")
            ),
            Granularity::Year => self.start.year().to_string(),
        }
    }
}

/// All periods overlapping `[start, end]` (inclusive), in order.
pub fn periods_covering(start: NaiveDate, end: NaiveDate, granularity: Granularity) -> Vec<Period> {
    let mut out = Vec::new();
    if end < start {
        return out;
    }
    let mut p = Period::containing(start, granularity);
    while p.start <= end {
        out.push(p);
        p = p.next();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn periods() {
        let w = Period::containing(d(2026, 11, 5), Granularity::Week); // Thursday
        assert_eq!(w.start, d(2026, 11, 2));
        assert_eq!(w.last_day(), d(2026, 11, 8));
        let q = Period::containing(d(2026, 8, 20), Granularity::Quarter);
        assert_eq!(q.start, d(2026, 7, 1));
        assert_eq!(q.label(), "Q3 2026");
        assert_eq!(q.next().start, d(2026, 10, 1));
        assert_eq!(q.prev().start, d(2026, 4, 1));
        let m = Period::containing(d(2026, 1, 31), Granularity::Month);
        assert_eq!(m.last_day(), d(2026, 1, 31));
        assert!(m.contains(d(2026, 1, 1)) && !m.contains(d(2026, 2, 1)));
        assert_eq!(
            periods_covering(d(2025, 12, 15), d(2026, 2, 1), Granularity::Month).len(),
            3
        );
        assert_eq!(
            periods_covering(d(2024, 6, 1), d(2026, 1, 1), Granularity::Year).len(),
            3
        );
    }
}
