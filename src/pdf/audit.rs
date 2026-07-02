//! Audit statement PDF layout (mirrors `templates/reports/audit.html`).

use super::PdfError;
use super::builder;
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

    for entry in &report.entries {
        builder::subheading(&mut doc, &format!("Entry #{}", entry.id));
        doc.push(builder::meta_line(
            "Posted",
            &entry.created_at.format("%Y-%m-%d %H:%M").to_string(),
        ));
        doc.push(builder::meta_line(
            "By",
            entry.posted_by_name.as_deref().unwrap_or("—"),
        ));
        doc.push(builder::meta_line("Source", &entry.source));

        let rows = entry
            .lines
            .iter()
            .map(|line| {
                vec![
                    line.entry_side.to_string(),
                    line.account_name.clone(),
                    builder::money(line.amount),
                    line.description.clone().unwrap_or_else(|| "—".to_string()),
                ]
            })
            .collect();

        builder::push_table(
            &mut doc,
            &[2, 4, 3, 4],
            &["Side", "Account", "Amount", "Description"],
            rows,
        );
        builder::total_line(
            &mut doc,
            "Total Debit / Credit",
            &format!(
                "{} / {}",
                builder::money(entry.total_debit),
                builder::money(entry.total_credit)
            ),
            true,
        );
        builder::spacer(&mut doc);
    }

    builder::render(doc)
}
