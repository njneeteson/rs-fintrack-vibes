//! Download the filtered transactions as CSV.

use super::filter::TxnQuery;
use crate::error::AppResult;
use crate::extract::HhCtx;
use crate::routes::attachment;
use crate::routes::hh::data::Data;
use axum::extract::Query;
use axum::response::Response;

fn write_csv(data: &Data, q: &TxnQuery) -> anyhow::Result<Vec<u8>> {
    let mut w = csv::Writer::from_writer(Vec::new());
    w.write_record([
        "Date",
        "Account",
        "Description",
        "Memo",
        "Amount",
        "Category",
        "Manual",
        "Note",
    ])?;
    for t in q.apply(data) {
        w.write_record([
            t.date.to_string(),
            data.account_name(t.account_id),
            t.description.clone(),
            t.memo.clone(),
            t.amount.to_plain(),
            t.category_id()
                .map(|c| data.cats.full_name(c))
                .unwrap_or_default(),
            if t.is_manual() {
                "yes".into()
            } else {
                String::new()
            },
            t.note.clone(),
        ])?;
    }
    Ok(w.into_inner()?)
}

pub async fn csv(h: HhCtx, Query(q): Query<TxnQuery>) -> AppResult<Response> {
    let data = h.hh.run(Data::load).await?;
    let bytes = write_csv(&data, &q)?;
    let file = format!(
        "{}-transactions-{}.csv",
        h.hh.info.slug,
        chrono::Local::now().format("%Y%m%d")
    );
    Ok(attachment(bytes, "text/csv; charset=utf-8", &file))
}
