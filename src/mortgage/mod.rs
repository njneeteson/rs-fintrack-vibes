//! Mortgage projections: amortization, progress to date, and scenario comparison.

pub mod amortization;

pub use amortization::{Scenario, Schedule, ScheduleRow, build_schedule, regular_payment};

use crate::model::{Money, Mortgage, Prepayment, RateChange};
use chrono::{Datelike, NaiveDate};

/// Build the scenario for a stored mortgage: its rate changes, enabled prepayments
/// and an optional extra-per-payment amount.
pub fn scenario_for(rates: &[RateChange], prepayments: &[Prepayment], extra_per_payment: Money) -> Scenario {
    Scenario {
        prepayments: prepayments.iter().filter(|p| p.enabled).map(|p| (p.date, p.amount)).collect(),
        rate_changes: rates.iter().map(|r| (r.effective_date, r.annual_rate_pct)).collect(),
        extra_per_payment,
    }
}

/// Where the mortgage stands on a given date according to the schedule.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Progress {
    pub payments_made: usize,
    pub payments_remaining: usize,
    pub principal_paid: Money,
    pub interest_paid: Money,
    pub balance: Money,
    /// Share of the original principal repaid (0..=1).
    pub fraction_repaid: f64,
}

pub fn progress_at(m: &Mortgage, sched: &Schedule, date: NaiveDate) -> Progress {
    let done: Vec<&ScheduleRow> = sched.rows.iter().filter(|r| r.date <= date).collect();
    let principal_paid: Money = done.iter().map(|r| r.principal + r.prepayment).sum();
    let balance = done.last().map_or(m.principal, |r| r.balance);
    Progress {
        payments_made: done.len(),
        payments_remaining: sched.rows.len() - done.len(),
        principal_paid,
        interest_paid: done.iter().map(|r| r.interest).sum(),
        balance,
        fraction_repaid: if m.principal.cents() > 0 { principal_paid.dollars() / m.principal.dollars() } else { 0.0 },
    }
}

/// Baseline (no prepayments / extra) vs. the scenario.
#[derive(Clone, Debug, PartialEq)]
pub struct Comparison {
    pub interest_saved: Money,
    pub payments_saved: i64,
    /// Months earlier the mortgage is paid off.
    pub months_saved: i64,
}

pub fn compare(baseline: &Schedule, scenario: &Schedule) -> Comparison {
    let months = |s: &Schedule| s.rows.last().map(|r| i64::from(r.date.year()) * 12 + i64::from(r.date.month0()));
    Comparison {
        interest_saved: baseline.total_interest - scenario.total_interest,
        payments_saved: baseline.rows.len() as i64 - scenario.rows.len() as i64,
        months_saved: match (months(baseline), months(scenario)) {
            (Some(a), Some(b)) => a - b,
            _ => 0,
        },
    }
}

/// One row per calendar year.
#[derive(Clone, Debug, PartialEq)]
pub struct YearSummary {
    pub year: i32,
    pub payments: Money,
    pub interest: Money,
    pub principal: Money,
    pub prepayments: Money,
    pub end_balance: Money,
}

pub fn yearly_summary(sched: &Schedule) -> Vec<YearSummary> {
    let mut out: Vec<YearSummary> = Vec::new();
    for r in &sched.rows {
        if out.last().is_none_or(|y| y.year != r.date.year()) {
            out.push(YearSummary {
                year: r.date.year(),
                payments: Money::ZERO,
                interest: Money::ZERO,
                principal: Money::ZERO,
                prepayments: Money::ZERO,
                end_balance: r.balance,
            });
        }
        let y = out.last_mut().expect("just pushed");
        y.payments += r.payment;
        y.interest += r.interest;
        y.principal += r.principal;
        y.prepayments += r.prepayment;
        y.end_balance = r.balance;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PaymentFrequency;

    #[test]
    fn progress_and_comparison() {
        let m = Mortgage {
            principal: Money::from_dollars(300_000.0),
            annual_rate_pct: 4.0,
            amortization_months: 240,
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            frequency: PaymentFrequency::Monthly,
            ..Default::default()
        };
        let base = build_schedule(&m, &Scenario::default());
        let p = progress_at(&m, &base, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        assert_eq!(p.payments_made, 12);
        assert!(p.balance < m.principal && p.fraction_repaid > 0.0);
        assert_eq!(p.principal_paid + p.balance, m.principal);

        let pre = vec![Prepayment {
            id: 1,
            mortgage_id: 1,
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            amount: Money::from_dollars(20_000.0),
            note: String::new(),
            enabled: true,
        }];
        let s = build_schedule(&m, &scenario_for(&[], &pre, Money::ZERO));
        let c = compare(&base, &s);
        assert!(c.interest_saved.cents() > 0 && c.months_saved > 0);
        let years = yearly_summary(&s);
        assert_eq!(years[0].year, 2024);
        assert_eq!(years[1].prepayments, Money::from_dollars(20_000.0));
        assert_eq!(years.last().unwrap().end_balance, Money::ZERO);
    }
}
