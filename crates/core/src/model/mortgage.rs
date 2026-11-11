//! Mortgage definitions, prepayments and rate changes.

use super::Money;
use chrono::NaiveDate;

str_enum! {
    pub enum PaymentFrequency {
        Monthly => ("monthly", "Monthly (12/yr)"),
        SemiMonthly => ("semi_monthly", "Semi-monthly (24/yr)"),
        BiWeekly => ("biweekly", "Bi-weekly (26/yr)"),
        AcceleratedBiWeekly => ("accel_biweekly", "Accelerated bi-weekly (26/yr)"),
        Weekly => ("weekly", "Weekly (52/yr)"),
        AcceleratedWeekly => ("accel_weekly", "Accelerated weekly (52/yr)"),
    }
}

impl PaymentFrequency {
    pub fn per_year(self) -> u32 {
        match self {
            Self::Monthly => 12,
            Self::SemiMonthly => 24,
            Self::BiWeekly | Self::AcceleratedBiWeekly => 26,
            Self::Weekly | Self::AcceleratedWeekly => 52,
        }
    }

    /// Accelerated payments are the monthly payment divided by this number.
    pub fn accelerated_divisor(self) -> Option<f64> {
        match self {
            Self::AcceleratedBiWeekly => Some(2.0),
            Self::AcceleratedWeekly => Some(4.0),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mortgage {
    pub id: i64,
    pub name: String,
    pub principal: Money,
    /// Nominal annual rate in percent (e.g. `4.79`), compounded semi-annually.
    pub annual_rate_pct: f64,
    pub amortization_months: u32,
    /// Date the funds were advanced; the first payment is one period later.
    pub start_date: NaiveDate,
    pub frequency: PaymentFrequency,
    /// Use this payment instead of the calculated one.
    pub payment_override: Option<Money>,
    /// Length of the current term (informational; renewal date).
    pub term_months: u32,
    pub notes: String,
}

impl Default for Mortgage {
    fn default() -> Self {
        Self {
            id: 0,
            name: "My mortgage".into(),
            principal: Money::from_dollars(500_000.0),
            annual_rate_pct: 4.5,
            amortization_months: 25 * 12,
            start_date: chrono::Local::now().date_naive(),
            frequency: PaymentFrequency::Monthly,
            payment_override: None,
            term_months: 60,
            notes: String::new(),
        }
    }
}

/// A lump-sum payment against principal.
#[derive(Clone, Debug, PartialEq)]
pub struct Prepayment {
    pub id: i64,
    pub mortgage_id: i64,
    pub date: NaiveDate,
    pub amount: Money,
    pub note: String,
    /// Disabled prepayments are kept but excluded from projections (what-if toggling).
    pub enabled: bool,
}

/// The interest rate changes (e.g. at renewal) on `effective_date`.
#[derive(Clone, Debug, PartialEq)]
pub struct RateChange {
    pub id: i64,
    pub mortgage_id: i64,
    pub effective_date: NaiveDate,
    pub annual_rate_pct: f64,
}
