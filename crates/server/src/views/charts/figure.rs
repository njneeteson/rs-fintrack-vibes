//! A chart as page markup: a canvas carrying its data for Chart.js, and a data table.

use super::Chart;
use super::table::data_table;
use maud::{Markup, html};

/// The interactive chart (drawn by `assets/charts.js` from the canvas's
/// `data-chart` JSON) with its data table. `title` is the chart's accessible name.
pub fn figure(chart: &Chart, title: &str) -> Markup {
    if chart.labels.is_empty() {
        return html! {};
    }
    // An attribute rather than a <script> element: htmx drops scripts from
    // content it swaps in, and maud escapes attribute values.
    let spec = serde_json::to_string(chart).unwrap_or_else(|_| "{}".into());
    html! {
        figure class="chart-figure" {
            div class="chart-box" {
                canvas data-chart=(spec) role="img" aria-label=(title) {}
            }
            details class="chart-data" {
                summary { "Show as table" }
                (data_table(chart))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Color, Series};
    use super::*;

    #[test]
    fn embeds_data_and_table() {
        let c = Chart {
            labels: vec!["Jan".into(), "Feb".into()],
            bars: vec![
                Series::new("Income", vec![100.0, 200.0], Color::nth(0))
                    .with_links(vec![Some("/x?a=1".into()), None]),
            ],
            stacked: false,
            lines: vec![Series::new("\"><b>", vec![50.0, -1.0], Color::Other)],
        };
        let html = figure(&c, "Cash flow").into_string();
        assert!(html.contains("data-chart=\"{&quot;labels&quot;:[&quot;Jan&quot;"));
        assert!(html.contains("&quot;color&quot;:&quot;--s0&quot;"));
        assert!(html.contains("&quot;links&quot;:[&quot;/x?a=1&quot;,null]"));
        assert!(!html.contains("<b>"), "series names must be escaped");
        assert!(html.contains("<td class=\"num\">$200</td>"));
        assert!(figure(&Chart::default(), "x").into_string().is_empty());
    }
}
