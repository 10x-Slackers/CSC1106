//! Income statement PDF layout (mirrors `templates/reports/income_statement.html`).

use chrono::NaiveDateTime;

use super::PdfError;
use super::builder;
use crate::models::report::income_statement::{IncomeStatement, IncomeStatementLine};

/// Format the reporting period, using an em dash for open-ended bounds.
fn period(stmt: &IncomeStatement) -> String {
    let fmt =
        |d: Option<NaiveDateTime>| d.map_or_else(|| "—".to_string(), |d| d.date().to_string());
    format!("{} to {}", fmt(stmt.from), fmt(stmt.to))
}

/// Turn statement lines into (account, amount) table rows.
fn section_rows(lines: &[IncomeStatementLine]) -> Vec<Vec<String>> {
    lines
        .iter()
        .map(|l| vec![l.account_name.clone(), builder::money(l.amount)])
        .collect()
}

/// Render an income statement into PDF bytes.
pub fn render_income_statement(stmt: &IncomeStatement) -> Result<Vec<u8>, PdfError> {
    let mut doc = builder::new_document("Income Statement")?;

    builder::heading(&mut doc, "INCOME STATEMENT");
    builder::spacer(&mut doc);
    doc.push(builder::meta_line("Period", &period(stmt)));
    builder::spacer(&mut doc);

    builder::subheading(&mut doc, "Revenue");
    builder::push_table(
        &mut doc,
        &[6, 3],
        &["Account", "Amount"],
        section_rows(&stmt.revenue),
    );
    builder::table_total_gap(&mut doc);
    builder::total_line(
        &mut doc,
        "Total Revenue",
        &builder::money(stmt.total_revenue),
    );
    builder::spacer(&mut doc);

    builder::subheading(&mut doc, "Expenses");
    builder::push_table(
        &mut doc,
        &[6, 3],
        &["Account", "Amount"],
        section_rows(&stmt.expenses),
    );
    builder::table_total_gap(&mut doc);
    builder::total_line(
        &mut doc,
        "Total Expenses",
        &builder::money(stmt.total_expenses),
    );
    builder::table_total_gap(&mut doc);

    let label = if stmt.is_loss {
        "Net Loss"
    } else {
        "Net Profit"
    };
    builder::total_line(&mut doc, label, &builder::money(stmt.net_profit));

    builder::render(doc)
}
