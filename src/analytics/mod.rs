//! Pure analytics over in-memory transactions. Nothing here touches the database or UI.

pub mod aggregate;
pub mod balances;
pub mod budget;
pub mod flow;
pub mod period;
pub mod range;
pub mod recurring;
pub mod summary;

pub use aggregate::{Breakdown, BucketSeries, PeriodTotals, breakdown, income_vs_expense};
pub use flow::{BucketKey, Bucketing, Flow, Lookup};
pub use period::{Granularity, Period};
pub use range::{DateRange, RangePreset};
