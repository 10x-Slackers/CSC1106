//! Balance sheet PDF layout (mirrors `templates/reports/balance_sheet.html`).

use rust_decimal::Decimal;

use super::PdfError;
use super::builder;
use crate::models::report::balance_sheet::{BalanceSheet, BalanceSheetLine};

/// Turn balance sheet lines into (account, amount) table rows.
fn section_rows(lines: &[BalanceSheetLine]) -> Vec<Vec<String>> {
    lines
        .iter()
        .map(|l| vec![l.account_name.clone(), builder::money(l.amount)])
        .collect()
}

/// Push a titled section: subheading, table, and a total line.
fn section(
    doc: &mut genpdf::Document,
    title: &str,
    lines: &[BalanceSheetLine],
    total_label: &str,
    total: Decimal,
) {
    builder::subheading(doc, title);
    builder::push_table(doc, &[6, 3], &["Account", "Amount"], section_rows(lines));
    builder::table_total_gap(doc);
    builder::total_line(doc, total_label, &builder::money(total), true);
}

/// Render a balance sheet into PDF bytes.
pub fn render_balance_sheet(sheet: &BalanceSheet) -> Result<Vec<u8>, PdfError> {
    let mut doc = builder::new_document("Balance Sheet")?;

    builder::heading(&mut doc, "BALANCE SHEET");
    builder::spacer(&mut doc);
    let as_of = sheet
        .as_of
        .map_or_else(|| "—".to_string(), |d| d.date().to_string());
    doc.push(builder::meta_line("As of", &as_of));
    builder::spacer(&mut doc);

    section(
        &mut doc,
        "Assets",
        &sheet.assets,
        "Total Assets",
        sheet.total_assets,
    );
    builder::spacer(&mut doc);
    section(
        &mut doc,
        "Liabilities",
        &sheet.liabilities,
        "Total Liabilities",
        sheet.total_liabilities,
    );
    builder::spacer(&mut doc);
    section(
        &mut doc,
        "Equity",
        &sheet.equity,
        "Total Equity",
        sheet.total_equity,
    );
    builder::table_total_gap(&mut doc);
    builder::total_line(
        &mut doc,
        "Total Liabilities & Equity",
        &builder::money(sheet.total_liabilities_and_equity),
        true,
    );

    builder::render(doc)
}
