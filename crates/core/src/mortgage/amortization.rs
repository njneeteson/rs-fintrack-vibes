//! Amortization schedules with Canadian (semi-annual) compounding.
//!
//! Canadian fixed-rate mortgages quote a nominal annual rate compounded
//! semi-annually, so the effective rate per payment period is
//! `(1 + j/2)^(2/n) - 1` where `n` is payments per year. Accelerated
//! frequencies pay the monthly payment ÷ 2 (bi-weekly) or ÷ 4 (weekly).

use crate::model::{Money, Mortgage, PaymentFrequency};
use chrono::{Days, Months, NaiveDate};

/// Effective interest rate per payment period.
pub fn periodic_rate(annual_rate_pct: f64, payments_per_year: u32) -> f64 {
    (1.0 + annual_rate_pct / 100.0 / 2.0).powf(2.0 / f64::from(payments_per_year)) - 1.0
}

/// Level payment that amortizes `principal` over `periods` at `rate` per period.
pub fn level_payment(principal: f64, rate: f64, periods: u32) -> f64 {
    if periods == 0 {
        return principal;
    }
    if rate.abs() < 1e-12 {
        return principal / f64::from(periods);
    }
    principal * rate / (1.0 - (1.0 + rate).powi(-(periods as i32)))
}

/// Regular payment for a balance amortized over `remaining_months`.
pub fn regular_payment(
    freq: PaymentFrequency,
    annual_rate_pct: f64,
    balance: Money,
    remaining_months: u32,
) -> Money {
    let balance = balance.dollars();
    let payment = match freq.accelerated_divisor() {
        Some(div) => {
            level_payment(
                balance,
                periodic_rate(annual_rate_pct, 12),
                remaining_months,
            ) / div
        }
        None => {
            let n = freq.per_year();
            let periods = (f64::from(remaining_months) * f64::from(n) / 12.0).round() as u32;
            level_payment(balance, periodic_rate(annual_rate_pct, n), periods)
        }
    };
    Money::from_dollars(payment)
}

/// Date of the `k`-th payment (k ≥ 1).
pub fn payment_date(start: NaiveDate, freq: PaymentFrequency, k: u32) -> NaiveDate {
    match freq {
        PaymentFrequency::Monthly => start + Months::new(k),
        PaymentFrequency::SemiMonthly => {
            let base = start + Months::new(k / 2);
            if k % 2 == 1 {
                base + Days::new(15)
            } else {
                base
            }
        }
        PaymentFrequency::BiWeekly | PaymentFrequency::AcceleratedBiWeekly => {
            start + Days::new(14 * u64::from(k))
        }
        PaymentFrequency::Weekly | PaymentFrequency::AcceleratedWeekly => {
            start + Days::new(7 * u64::from(k))
        }
    }
}

/// Inputs that vary between projection scenarios.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scenario {
    /// Lump-sum prepayments `(date, amount)`.
    pub prepayments: Vec<(NaiveDate, Money)>,
    /// Rate changes `(effective date, annual rate %)`.
    pub rate_changes: Vec<(NaiveDate, f64)>,
    /// Extra amount added to every regular payment.
    pub extra_per_payment: Money,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleRow {
    pub number: u32,
    pub date: NaiveDate,
    pub rate_pct: f64,
    /// Regular payment + extra (capped at what is owed).
    pub payment: Money,
    pub interest: Money,
    pub principal: Money,
    pub prepayment: Money,
    pub balance: Money,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Schedule {
    pub rows: Vec<ScheduleRow>,
    pub total_interest: Money,
    pub total_paid: Money,
    /// The payment no longer covers interest (e.g. after a large rate increase with a
    /// fixed payment).
    pub negative_amortization: bool,
}

impl Schedule {
    pub fn payoff_date(&self) -> Option<NaiveDate> {
        self.rows
            .last()
            .filter(|r| r.balance.is_zero())
            .map(|r| r.date)
    }

    pub fn first_payment(&self) -> Option<Money> {
        self.rows.first().map(|r| r.payment)
    }
}

/// Build the full schedule for `m` under `scenario`.
///
/// Prepayments are applied right after the regular payment on or after their
/// date and shorten the amortization (the regular payment is unchanged). At a
/// rate change the payment is recalculated over the *remaining* amortization —
/// how long the current balance would take at the old rate and payment — so
/// time saved by earlier prepayments is kept, unless the mortgage has a
/// payment override.
pub fn build_schedule(m: &Mortgage, scenario: &Scenario) -> Schedule {
    let mut sched = Schedule::default();
    let mut balance = m.principal;
    if balance.cents() <= 0 || m.amortization_months == 0 {
        return sched;
    }
    let mut rate = m.annual_rate_pct;
    let mut rate_changes = scenario.rate_changes.clone();
    rate_changes.sort_by_key(|r| r.0);
    let mut rate_iter = rate_changes.into_iter().peekable();
    let mut prepayments = scenario.prepayments.clone();
    prepayments.sort_by_key(|p| p.0);
    let mut pre_iter = prepayments.into_iter().peekable();

    let amort_end = m.start_date + Months::new(m.amortization_months);
    let ppy = m.frequency.per_year();
    // Months the balance needs at `rate` and `payment`; falls back to the original
    // end date if the payment doesn't cover the interest.
    let months_left = |d: NaiveDate, balance: Money, rate: f64, payment: Money| -> u32 {
        let r = periodic_rate(rate, ppy);
        let (b, p) = (balance.dollars(), payment.dollars());
        let months = if r > 0.0 && p > r * b {
            -(1.0 - r * b / p).ln() / (1.0 + r).ln() * 12.0 / f64::from(ppy)
        } else {
            (amort_end - d).num_days() as f64 / 30.436_875
        };
        months.round().max(1.0) as u32
    };
    let mut payment = m
        .payment_override
        .unwrap_or_else(|| regular_payment(m.frequency, rate, balance, m.amortization_months));
    let max_payments = m.frequency.per_year() * (m.amortization_months / 12 + 1) * 3;

    for k in 1..=max_payments {
        let date = payment_date(m.start_date, m.frequency, k);
        let old_rate = rate;
        let mut rate_changed = false;
        while let Some(&(d, r)) = rate_iter.peek() {
            if d > date {
                break;
            }
            rate = r;
            rate_changed = true;
            rate_iter.next();
        }
        if rate_changed && m.payment_override.is_none() {
            let remaining = months_left(date, balance, old_rate, payment);
            payment = regular_payment(m.frequency, rate, balance, remaining);
        }
        let interest =
            Money::from_dollars(balance.dollars() * periodic_rate(rate, m.frequency.per_year()));
        let due = balance + interest;
        let mut paid = (payment + scenario.extra_per_payment).min(due);
        // Cent rounding leaves a tiny residue at the end; the final payment absorbs it.
        let residue_tolerance = Money((payment.cents() / 100).max(100));
        if due - paid <= residue_tolerance {
            paid = due;
        }
        if paid <= interest && paid < due {
            sched.negative_amortization = true;
        }
        let principal = paid - interest;
        balance = due - paid;
        let mut prepaid = Money::ZERO;
        while let Some(&(d, amt)) = pre_iter.peek() {
            if d > date {
                break;
            }
            let applied = amt.abs().min(balance);
            prepaid += applied;
            balance -= applied;
            pre_iter.next();
        }
        sched.total_interest += interest;
        sched.total_paid += paid + prepaid;
        sched.rows.push(ScheduleRow {
            number: k,
            date,
            rate_pct: rate,
            payment: paid,
            interest,
            principal,
            prepayment: prepaid,
            balance,
        });
        if balance.cents() <= 0 || sched.negative_amortization {
            break;
        }
    }
    sched
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mortgage(freq: PaymentFrequency) -> Mortgage {
        Mortgage {
            principal: Money::from_dollars(500_000.0),
            annual_rate_pct: 5.0,
            amortization_months: 300,
            start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            frequency: freq,
            ..Default::default()
        }
    }

    #[test]
    fn canadian_monthly_payment() {
        // Well-known figure: $500k, 5% semi-annual, 25 yrs ≈ $2,908.02/month.
        let p = regular_payment(
            PaymentFrequency::Monthly,
            5.0,
            Money::from_dollars(500_000.0),
            300,
        );
        assert!((p.dollars() - 2908.02).abs() < 0.05, "{p}");
        let s = build_schedule(&mortgage(PaymentFrequency::Monthly), &Scenario::default());
        assert_eq!(s.rows.len(), 300);
        assert!(s.payoff_date().is_some());
        assert_eq!(s.rows[0].date, NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
    }

    #[test]
    fn accelerated_and_prepayments_shorten() {
        let base = build_schedule(&mortgage(PaymentFrequency::Monthly), &Scenario::default());
        let accel = build_schedule(
            &mortgage(PaymentFrequency::AcceleratedBiWeekly),
            &Scenario::default(),
        );
        assert!(accel.payoff_date() < base.payoff_date());
        assert!(accel.total_interest < base.total_interest);

        let scen = Scenario {
            prepayments: vec![(
                NaiveDate::from_ymd_opt(2027, 1, 15).unwrap(),
                Money::from_dollars(50_000.0),
            )],
            ..Default::default()
        };
        let pre = build_schedule(&mortgage(PaymentFrequency::Monthly), &scen);
        assert!(pre.rows.len() < base.rows.len());
        assert!(pre.total_interest < base.total_interest);
        assert_eq!(pre.rows[12].prepayment, Money::from_dollars(50_000.0));
    }

    #[test]
    fn rate_change_recalculates_payment() {
        let scen = Scenario {
            rate_changes: vec![(NaiveDate::from_ymd_opt(2031, 1, 1).unwrap(), 7.0)],
            ..Default::default()
        };
        let s = build_schedule(&mortgage(PaymentFrequency::Monthly), &scen);
        let before = s.rows[0].payment;
        let after = s.rows.iter().find(|r| r.rate_pct == 7.0).unwrap().payment;
        assert!(after > before);
        assert!(s.payoff_date().unwrap() <= NaiveDate::from_ymd_opt(2051, 2, 1).unwrap());
    }

    #[test]
    fn prepayment_before_renewal_still_shortens_payoff() {
        let renewal = vec![(NaiveDate::from_ymd_opt(2031, 1, 1).unwrap(), 4.0)];
        let base = build_schedule(
            &mortgage(PaymentFrequency::Monthly),
            &Scenario {
                rate_changes: renewal.clone(),
                ..Default::default()
            },
        );
        let with_pre = build_schedule(
            &mortgage(PaymentFrequency::Monthly),
            &Scenario {
                rate_changes: renewal,
                prepayments: vec![(
                    NaiveDate::from_ymd_opt(2028, 6, 1).unwrap(),
                    Money::from_dollars(40_000.0),
                )],
                ..Default::default()
            },
        );
        assert!(with_pre.payoff_date().unwrap() < base.payoff_date().unwrap());
        // Without prepayments the renewal keeps the original 25-year end date (± a payment).
        let end = base.payoff_date().unwrap();
        assert!(
            end >= NaiveDate::from_ymd_opt(2050, 12, 1).unwrap()
                && end <= NaiveDate::from_ymd_opt(2051, 2, 1).unwrap(),
            "{end}"
        );
    }

    #[test]
    fn semi_monthly_dates() {
        let s = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        assert_eq!(
            payment_date(s, PaymentFrequency::SemiMonthly, 1),
            NaiveDate::from_ymd_opt(2026, 1, 16).unwrap()
        );
        assert_eq!(
            payment_date(s, PaymentFrequency::SemiMonthly, 2),
            NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()
        );
    }
}
