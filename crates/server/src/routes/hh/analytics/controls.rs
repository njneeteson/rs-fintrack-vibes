//! Analytics controls (range, period size, flow, grouping, series count),
//! carried in the query string and shown as one filter row.

use super::{CONTENT, TABS, Tab};
use crate::extract::HhCtx;
use crate::routes::hh::data::Data;
use crate::routes::hh::range::RangeQuery;
use crate::views::charts::MAX_SERIES;
use crate::views::forms::{Live, LiveTarget, enum_select, field, live_form, number_input};
use fintrack_core::analytics::{BucketKey, Bucketing, DateRange, Flow, Granularity, RangePreset};
use fintrack_core::help::tips;
use maud::{Markup, html};
use serde::{Deserialize, Serialize};

const DEFAULT_RANGE: RangePreset = RangePreset::Last12Months;
const DEFAULT_TOP: usize = MAX_SERIES - 1;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Controls {
    #[serde(flatten)]
    pub range: RangeQuery,
    pub g: String,
    pub flow: String,
    pub by: String,
    pub top: String,
    /// Trends: the chosen bucket, see [`encode_bucket`].
    pub bucket: String,
}

impl Controls {
    pub fn granularity(&self) -> Granularity {
        Granularity::from_key(&self.g).unwrap_or(Granularity::Month)
    }

    pub fn flow(&self) -> Flow {
        Flow::from_key(&self.flow).unwrap_or(Flow::Expense)
    }

    pub fn bucketing(&self) -> Bucketing {
        Bucketing::from_key(&self.by).unwrap_or(Bucketing::TopCategory)
    }

    /// Series drawn before the rest fold into "Other" (one colour is kept for "Other").
    pub fn top_n(&self) -> usize {
        self.top
            .parse()
            .unwrap_or(DEFAULT_TOP)
            .clamp(1, DEFAULT_TOP)
    }

    pub fn date_range(&self, data: &Data) -> DateRange {
        self.range.resolve(DEFAULT_RANGE, data.bounds())
    }

    /// The filter row for `tab`, showing only the controls it uses.
    pub fn form(&self, h: &HhCtx, tab: Tab, data: &Data) -> Markup {
        let uses_range = tab != Tab::Recurring;
        let uses_period = !matches!(tab, Tab::Payees | Tab::Recurring);
        let uses_flow = matches!(tab, Tab::Breakdown | Tab::Trends | Tab::Payees);
        let uses_bucketing = matches!(tab, Tab::Breakdown | Tab::Trends);
        let max_top = DEFAULT_TOP.to_string();
        let controls = html! {
            @if uses_range { (self.range.controls(DEFAULT_RANGE, data.bounds())) }
            @if uses_period {
                (field("Period", tips::GRANULARITY, enum_select("g", self.granularity(), true)))
            }
            @if uses_flow {
                (field("Show", tips::FLOW, enum_select("flow", self.flow(), true)))
            }
            @if uses_bucketing {
                (field("Group by", tips::BUCKETING, enum_select("by", self.bucketing(), true)))
            }
            @if tab == Tab::Breakdown {
                (field("Series", tips::TOP_N,
                    number_input("top", self.top_n(), ("1", &max_top), "1")))
            }
            @if !self.bucket.is_empty() {
                input type="hidden" name="bucket" value=(self.bucket);
            }
            @if uses_range { noscript { button type="submit" class="primary" { "Apply" } } }
        };
        let action = h.at(&format!("/analytics/{}", tab.key()));
        let target = LiveTarget {
            id: CONTENT,
            also: &[TABS],
        };
        live_form(&action, "toolbar", target, Live::Filter, controls)
    }
}

/// A bucket as a query value: `c12`, `a3`, `u`, `pSTARBUCKS`.
pub fn encode_bucket(key: &BucketKey) -> String {
    match key {
        BucketKey::Category(id) => format!("c{id}"),
        BucketKey::Account(id) => format!("a{id}"),
        BucketKey::Uncategorized => "u".into(),
        BucketKey::Payee(p) => format!("p{p}"),
        BucketKey::Other => "o".into(),
    }
}

pub fn decode_bucket(s: &str) -> Option<BucketKey> {
    let (tag, rest) = s.split_at_checked(1)?;
    match tag {
        "c" => rest.parse().ok().map(BucketKey::Category),
        "a" => rest.parse().ok().map(BucketKey::Account),
        "u" => Some(BucketKey::Uncategorized),
        "p" => Some(BucketKey::Payee(rest.to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_codes() {
        for k in [
            BucketKey::Category(12),
            BucketKey::Account(3),
            BucketKey::Uncategorized,
            BucketKey::Payee("TIM".into()),
        ] {
            assert_eq!(decode_bucket(&encode_bucket(&k)), Some(k));
        }
        assert_eq!(decode_bucket(""), None);
    }
}
