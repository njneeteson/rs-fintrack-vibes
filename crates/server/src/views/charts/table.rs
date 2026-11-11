//! A chart's data as a table: an accessible alternative, and what the chart
//! shows when JavaScript is off.

use super::Chart;
use fintrack_core::model::Money;
use maud::{Markup, html};

fn cell(v: Option<&f64>) -> String {
    v.map(|v| Money::from_dollars(*v).to_whole())
        .unwrap_or_default()
}

pub fn data_table(chart: &Chart) -> Markup {
    let series: Vec<_> = chart.all_series().collect();
    html! {
        div class="table-wrap" {
            table {
                thead {
                    tr {
                        th {}
                        @for s in &series { th class="num" { (s.name) } }
                    }
                }
                tbody {
                    @for (i, label) in chart.labels.iter().enumerate() {
                        tr {
                            th { (label) }
                            @for s in &series { td class="num" { (cell(s.values.get(i))) } }
                        }
                    }
                }
            }
        }
    }
}
