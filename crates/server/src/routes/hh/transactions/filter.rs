//! Transaction filters, carried in the query string so a filtered list is a
//! shareable URL (and analytics can link to "the transactions behind this bar").

use crate::extract::HhCtx;
use crate::form::opt_id;
use crate::routes::hh::data::Data;
use crate::routes::hh::range::RangeQuery;
use crate::views::forms::{
    Live, LiveTarget, account_select, category_options, checkbox, field, live_form,
};
use fintrack_core::analytics::{BucketKey, Bucketing, DateRange, Period, RangePreset};
use fintrack_core::model::{CategoryTree, Transaction};
use maud::{Markup, html};
use serde::{Deserialize, Serialize};

const DEFAULT_RANGE: RangePreset = RangePreset::AllTime;
/// `cat=` value meaning "no category".
const UNCATEGORIZED: &str = "none";
/// Id of the results area the filter form refreshes.
pub const RESULTS: &str = "txn-results";
const CATEGORY_TIP: &str =
    "A top-level category includes its subcategories unless you tick “exact”.";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TxnQuery {
    #[serde(flatten)]
    pub range: RangeQuery,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub account: String,
    /// `""` any, `"none"` uncategorized, or a category id.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cat: String,
    /// `"1"`: only the chosen category itself, not its subcategories.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub exact: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub q: String,
    /// `"1"`: only transactions with a manual category override.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub manual: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub page: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CategoryFilter {
    Any,
    Uncategorized,
    Category { id: i64, include_children: bool },
}

impl TxnQuery {
    pub fn uncategorized() -> Self {
        Self {
            cat: UNCATEGORIZED.into(),
            ..Default::default()
        }
    }

    /// The transactions behind one bucket of an analytics chart.
    pub fn for_bucket(key: &BucketKey, bucketing: Bucketing, range: DateRange) -> Option<Self> {
        let mut q = Self {
            range: RangeQuery::custom(range),
            ..Default::default()
        };
        match key {
            BucketKey::Category(id) => {
                q.cat = id.to_string();
                if bucketing == Bucketing::SubCategory {
                    q.exact = "1".into();
                }
            }
            BucketKey::Uncategorized => q.cat = UNCATEGORIZED.into(),
            BucketKey::Account(id) => q.account = id.to_string(),
            BucketKey::Payee(p) => q.q = p.to_lowercase(),
            BucketKey::Other => return None,
        }
        Some(q)
    }

    pub fn category(&self) -> CategoryFilter {
        match (self.cat.as_str(), opt_id(&self.cat)) {
            (UNCATEGORIZED, _) => CategoryFilter::Uncategorized,
            (_, Some(id)) => CategoryFilter::Category {
                id,
                include_children: self.exact.is_empty(),
            },
            _ => CategoryFilter::Any,
        }
    }

    pub fn page(&self) -> usize {
        self.page.parse().unwrap_or(1).max(1)
    }

    pub fn with_page(&self, page: usize) -> Self {
        Self {
            page: if page <= 1 {
                String::new()
            } else {
                page.to_string()
            },
            ..self.clone()
        }
    }

    /// `?…` (empty when there are no filters).
    pub fn to_query(&self) -> String {
        let s = serde_html_form::to_string(self).unwrap_or_default();
        if s.is_empty() { s } else { format!("?{s}") }
    }

    /// Link to the Transactions page with these filters.
    pub fn href(&self, h: &HhCtx) -> String {
        format!("{}{}", h.at("/transactions"), self.to_query())
    }

    /// For each period, a link to that bucket's transactions in it (within `range`),
    /// e.g. for clicking a bar in a chart.
    pub fn period_hrefs(
        h: &HhCtx,
        key: &BucketKey,
        bucketing: Bucketing,
        periods: &[Period],
        range: DateRange,
    ) -> Vec<Option<String>> {
        periods
            .iter()
            .map(|p| {
                let start = p.start.max(range.start);
                let end = p.last_day().min(range.end);
                Self::bucket_href(h, key, bucketing, DateRange::new(start, end))
            })
            .collect()
    }

    /// Link to the transactions behind one analytics bucket (`None` for "Other").
    pub fn bucket_href(
        h: &HhCtx,
        key: &BucketKey,
        bucketing: Bucketing,
        range: DateRange,
    ) -> Option<String> {
        Self::for_bucket(key, bucketing, range).map(|q| q.href(h))
    }

    pub fn date_range(&self, data: &Data) -> DateRange {
        self.range.resolve(DEFAULT_RANGE, data.bounds())
    }

    fn matches(
        &self,
        t: &Transaction,
        cats: &CategoryTree,
        range: DateRange,
        needle: &str,
    ) -> bool {
        let account = opt_id(&self.account);
        range.contains(t.date)
            && account.is_none_or(|a| a == t.account_id)
            && (self.manual.is_empty() || t.is_manual())
            && match self.category() {
                CategoryFilter::Any => true,
                CategoryFilter::Uncategorized => t.is_uncategorized(),
                CategoryFilter::Category {
                    id,
                    include_children,
                } => t.category_id().is_some_and(|c| {
                    if include_children {
                        cats.is_under(c, id)
                    } else {
                        c == id
                    }
                }),
            }
            && (needle.is_empty()
                || [&t.description, &t.memo, &t.note]
                    .iter()
                    .any(|s| s.to_lowercase().contains(needle)))
    }

    /// The matching transactions, newest first.
    pub fn apply<'a>(&self, data: &'a Data) -> Vec<&'a Transaction> {
        let range = self.date_range(data);
        let needle = self.q.trim().to_lowercase();
        data.txns
            .iter()
            .filter(|t| self.matches(t, &data.cats, range, &needle))
            .collect()
    }

    fn category_picker(&self, cats: &CategoryTree) -> Markup {
        html! {
            select name="cat" {
                option value="" selected[self.cat.is_empty()] { "Any category" }
                option value=(UNCATEGORIZED) selected[self.cat == UNCATEGORIZED] { "Uncategorized" }
                (category_options(cats, opt_id(&self.cat)))
            }
        }
    }

    /// The filter form (GET, so the result is a link).
    pub fn controls(&self, h: &HhCtx, data: &Data) -> Markup {
        let accounts = account_select(
            "account",
            &data.accounts,
            opt_id(&self.account),
            Some("All"),
            false,
        );
        let controls = html! {
            (self.range.controls(DEFAULT_RANGE, data.bounds()))
            (field("Account", "", accounts))
            (field("Category", CATEGORY_TIP, self.category_picker(&data.cats)))
            (checkbox("exact", !self.exact.is_empty(), html! { "exact" }))
            (field("Search", "Description, memo or note. Press / to jump here.", html! {
                input type="search" name="q" value=(self.q) placeholder="e.g. costco";
            }))
            (checkbox("manual", !self.manual.is_empty(), html! { "Manual overrides only" }))
            noscript { button type="submit" class="primary" { "Apply" } }
            a class="btn" href=(h.at("/transactions")) { "Reset" }
        };
        let target = LiveTarget {
            id: RESULTS,
            also: &[],
        };
        live_form(
            &h.at("/transactions"),
            "toolbar",
            target,
            Live::Filter,
            controls,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_round_trip() {
        let q = TxnQuery {
            cat: "12".into(),
            q: "save on".into(),
            ..Default::default()
        };
        let s = q.to_query();
        assert_eq!(s, "?cat=12&q=save+on");
        let back: TxnQuery = serde_html_form::from_str(&s[1..]).unwrap();
        assert_eq!(back, q);
        assert_eq!(
            back.category(),
            CategoryFilter::Category {
                id: 12,
                include_children: true
            }
        );
        assert_eq!(TxnQuery::default().to_query(), "");
        assert_eq!(
            TxnQuery::uncategorized().category(),
            CategoryFilter::Uncategorized
        );
    }
}
