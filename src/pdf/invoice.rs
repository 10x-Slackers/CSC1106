//! Invoice PDF layout (mirrors `templates/invoices/show.html`).

use super::PdfError;
use super::builder;
use crate::models::invoice::{Invoice, InvoiceLineItem, grand_total, gst_total, subtotal};
use crate::models::party::Party;

/// Render an invoice, its line items, and optional party into PDF bytes.
pub fn render_invoice(
    invoice: &Invoice,
    items: &[InvoiceLineItem],
    party: Option<&Party>,
) -> Result<Vec<u8>, PdfError> {
    let mut doc = builder::new_document(&format!("Invoice {}", invoice.invoice_no))?;

    builder::heading(&mut doc, &format!("Invoice #{}", invoice.invoice_no));
    builder::spacer(&mut doc);

    // Party name and address (values only, no labels).
    doc.push(builder::text_line(
        party.map(|p| p.name.as_str()).unwrap_or("—"),
    ));
    if let Some(p) = party
        && !p.address.is_empty()
    {
        doc.push(builder::text_line(&p.address));
    }
    builder::spacer(&mut doc);

    doc.push(builder::meta_line(
        "Issue Date",
        &invoice.issue_date.to_string(),
    ));
    doc.push(builder::meta_line(
        "Due Date",
        &invoice.due_date.to_string(),
    ));
    builder::spacer(&mut doc);

    let rows = items
        .iter()
        .map(|item| {
            vec![
                item.description.clone(),
                item.quantity.to_string(),
                builder::money(item.unit_price),
                builder::money(item.line_gst_amount),
                builder::money(item.line_total_with_gst),
            ]
        })
        .collect();

    builder::push_table(
        &mut doc,
        &[5, 2, 3, 4, 4],
        &["Description", "Qty", "Unit Price", "GST", "Total w/ GST"],
        rows,
    );
    builder::spacer(&mut doc);

    builder::total_line(
        &mut doc,
        "Subtotal",
        &builder::money(subtotal(items)),
        false,
    );
    builder::table_total_gap(&mut doc);
    builder::total_line(
        &mut doc,
        "GST Total",
        &builder::money(gst_total(items)),
        false,
    );
    builder::table_total_gap(&mut doc);
    builder::total_line(
        &mut doc,
        "Grand Total",
        &builder::money(grand_total(items)),
        true,
    );

    builder::render(doc)
}
