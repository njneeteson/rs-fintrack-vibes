//! Date-range selection shared by Transactions and Analytics: a preset
//! ("Last 12 months") or a custom from/to, carried in the query string.

use super::data::today;
use crate::form::opt_date;
use crate::views::forms::{enum_select, field};
use fintrack_core::analytics::{DateRange, RangePreset};
use maud::{Markup, html};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RangeQuery {
    /// A [`RangePreset`] key; empty means the page's default.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub range: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub from: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub to: String,
}

impl RangeQuery {
    /// A custom range (used by drill-down links).
    pub fn custom(r: DateRange) -> Self {
        Self {
            range: RangePreset::Custom.key().into(),
            from: r.start.to_string(),
            to: r.end.to_string(),
        }
    }

    pub fn preset(&self, default: RangePreset) -> RangePreset {
        RangePreset::from_key(&self.range).unwrap_or(default)
    }

    pub fn resolve(&self, default: RangePreset, bounds: Option<DateRange>) -> DateRange {
        let t = today();
        let fallback = bounds.unwrap_or(DateRange::new(t, t));
        let custom = DateRange::new(
            opt_date(&self.from).unwrap_or(fallback.start),
            opt_date(&self.to).unwrap_or(fallback.end),
        );
        self.preset(default).resolve(t, bounds, custom)
    }

    /// Preset picker plus from/to inputs (used when "Custom range" is chosen).
    pub fn controls(&self, default: RangePreset, bounds: Option<DateRange>) -> Markup {
        let shown = self.resolve(default, bounds);
        // Typing a date switches the preset to "Custom range" (app.js).
        let date = |name: &str, d: chrono::NaiveDate| {
            html! {
                input type="date" name=(name) value=(d.to_string()) data-implies="range=custom";
            }
        };
        html! {
            (field("Dates", "Choose “Custom range” to enter your own dates.",
                enum_select("range", self.preset(default), true)))
            (field("From", "", date("from", shown.start)))
            (field("To", "", date("to", shown.end)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn custom_and_presets() {
        let d = |m, day| NaiveDate::from_ymd_opt(2026, m, day).unwrap();
        let q = RangeQuery::custom(DateRange::new(d(1, 5), d(2, 10)));
        assert_eq!(
            q.resolve(RangePreset::AllTime, None),
            DateRange::new(d(1, 5), d(2, 10))
        );
        let all = RangeQuery::default()
            .resolve(RangePreset::AllTime, Some(DateRange::new(d(1, 1), d(3, 1))));
        assert_eq!(all, DateRange::new(d(1, 1), d(3, 1)));
    }
}
