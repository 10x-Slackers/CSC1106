//! GST summary PDF layout (mirrors `templates/reports/gst.html`).
//!
//! Authors: Felicia

use chrono::NaiveDate;

use super::PdfError;
use super::builder;
use crate::models::report::gst::GstSummary;

/// Format the reporting period, using an em dash for open-ended bounds.
fn period(from: Option<NaiveDate>, to: Option<NaiveDate>) -> String {
    let fmt = |d: Option<NaiveDate>| d.map_or_else(|| "—".to_string(), |d| d.to_string());
    format!("{} to {}", fmt(from), fmt(to))
}

/// Render a GST summary into PDF bytes.
pub fn render_gst(summary: &GstSummary) -> Result<Vec<u8>, PdfError> {
    let mut doc = builder::new_document("GST Summary")?;

    builder::heading(&mut doc, "GST SUMMARY");
    builder::spacer(&mut doc);
    doc.push(builder::meta_line(
        "Period",
        &period(summary.from, summary.to),
    ));
    builder::spacer(&mut doc);

    let rows = summary
        .invoices
        .iter()
        .map(|inv| {
            vec![
                inv.invoice_no.clone(),
                inv.issue_date.to_string(),
                builder::money(inv.gst_amount),
            ]
        })
        .collect();

    builder::push_table(
        &mut doc,
        &[4, 3, 3],
        &["Invoice No", "Issue Date", "GST Amount"],
        rows,
    );
    builder::table_total_gap(&mut doc);
    builder::total_line(&mut doc, "Total", &builder::money(summary.total_gst), true);

    builder::render(doc)
}
