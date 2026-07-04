//! Audit statement PDF layout
//!
//! Authors: Felicia

use rust_decimal::Decimal;

use super::PdfError;
use super::builder;
use crate::entity::journal_entry_line::EntrySide;
use crate::models::report::audit::AuditReport;

/// Render an audit statement into PDF bytes.
pub fn render_audit(report: &AuditReport) -> Result<Vec<u8>, PdfError> {
    let mut doc = builder::new_document("Audit Statement")?;

    builder::heading(&mut doc, "AUDIT STATEMENT");
    builder::spacer(&mut doc);
    doc.push(builder::meta_line("Year", &report.year.to_string()));
    doc.push(builder::meta_line(
        "Entries",
        &report.total_entries.to_string(),
    ));
    doc.push(builder::meta_line(
        "Period",
        &format!("{} to {}", report.from.date(), report.to.date()),
    ));
    builder::spacer(&mut doc);

    // One continuous ledger: each journal line is a row, with the entry's date
    // and source shown only on its first line so entries read as groups.
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut total_debit = Decimal::ZERO;
    let mut total_credit = Decimal::ZERO;

    for entry in &report.entries {
        for (i, line) in entry.lines.iter().enumerate() {
            let (debit, credit) = match line.entry_side {
                EntrySide::Debit => (builder::money(line.amount), String::new()),
                EntrySide::Credit => (String::new(), builder::money(line.amount)),
            };
            let (date, source) = if i == 0 {
                (entry.created_at.date().to_string(), entry.source.clone())
            } else {
                (String::new(), String::new())
            };
            rows.push(vec![date, source, line.account_name.clone(), debit, credit]);
        }
        total_debit += entry.total_debit;
        total_credit += entry.total_credit;
    }

    builder::push_table_aligned(
        &mut doc,
        &[3, 4, 4, 3, 3],
        &["Date", "Source", "Account", "Debit", "Credit"],
        rows,
        3, // right-align Debit & Credit
    );
    builder::table_total_gap(&mut doc);

    builder::total_line(&mut doc, "Total Debit", &builder::money(total_debit), true);
    builder::table_total_gap(&mut doc);
    builder::total_line(
        &mut doc,
        "Total Credit",
        &builder::money(total_credit),
        true,
    );

    builder::render(doc)
}
