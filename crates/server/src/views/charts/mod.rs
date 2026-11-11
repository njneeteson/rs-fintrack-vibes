//! Charts: the server describes them, Chart.js draws them in the browser.
//!
//! A [`Chart`] is rendered by [`figure`] as a `<canvas>` plus its data as JSON;
//! `assets/charts.js` turns that into an interactive Chart.js chart (tooltips
//! for every series at a point, clickable legend, click-through links). The
//! same data is also shown as a table, which is what you get without JavaScript.
//!
//! Colours are CSS custom properties (`--s0`…`--s7`, `--other`), so light and
//! dark mode use the validated palette steps. Series are coloured by identity in
//! a fixed order; anything past the eighth series is folded into "Other" by the
//! caller.

mod figure;
mod table;

pub use figure::figure;

use fintrack_core::model::Money;
use maud::{Markup, html};
use serde::Serialize;

/// Number of distinct series colours.
pub const MAX_SERIES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Series(usize),
    Other,
}

impl Color {
    /// Colour for the `i`-th series in fixed order.
    pub fn nth(i: usize) -> Self {
        if i < MAX_SERIES {
            Color::Series(i)
        } else {
            Color::Other
        }
    }

    /// CSS class for HTML swatches and bars (`c0`, `cother`).
    pub fn class(self) -> String {
        match self {
            Color::Series(i) if i < MAX_SERIES => format!("c{i}"),
            _ => "cother".into(),
        }
    }

    /// CSS custom property holding the colour (`--s0`, `--other`).
    pub fn var(self) -> String {
        match self {
            Color::Series(i) if i < MAX_SERIES => format!("--s{i}"),
            _ => "--other".into(),
        }
    }
}

impl Serialize for Color {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.var())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Series {
    pub name: String,
    /// Dollar values, one per label.
    pub values: Vec<f64>,
    pub color: Color,
    /// Optional link per value: clicking that bar or point opens it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Option<String>>,
}

impl Series {
    pub fn new(name: impl Into<String>, values: Vec<f64>, color: Color) -> Self {
        Self {
            name: name.into(),
            values,
            color,
            links: Vec::new(),
        }
    }

    pub fn from_money(name: impl Into<String>, values: &[Money], color: Color) -> Self {
        Self::new(name, values.iter().map(|m| m.dollars()).collect(), color)
    }

    /// Make each value clickable.
    pub fn with_links(mut self, links: Vec<Option<String>>) -> Self {
        self.links = links;
        self
    }
}

/// A chart over labelled x positions: bars (grouped or stacked) and/or lines.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Chart {
    pub labels: Vec<String>,
    pub bars: Vec<Series>,
    pub stacked: bool,
    pub lines: Vec<Series>,
}

impl Chart {
    fn all_series(&self) -> impl Iterator<Item = &Series> {
        self.bars.iter().chain(&self.lines)
    }
}

/// One row of a [`bar_list`].
pub struct BarItem {
    pub label: String,
    pub value: Money,
    pub href: Option<String>,
    pub color: Color,
}

/// Horizontal bars with the label and amount as text (reads well on phones).
pub fn bar_list(items: &[BarItem]) -> Markup {
    let max = items.iter().map(|i| i.value.dollars()).fold(0.0, f64::max);
    let width = |v: Money| {
        if max > 0.0 {
            (v.dollars().max(0.0) / max * 100.0).round()
        } else {
            0.0
        }
    };
    html! {
        div class="bar-list" {
            @for it in items {
                @let width = width(it.value);
                span class="label" title=(it.label) {
                    @match &it.href {
                        Some(href) => a href=(href) { (it.label) },
                        None => (it.label),
                    }
                }
                span class="bar" { span class=(it.color.class()) style={ "width:" (width) "%" } {} }
                span class="num" { (it.value.to_whole()) }
            }
        }
    }
}
