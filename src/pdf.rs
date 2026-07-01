//! `genpdf` invoice generator (approach 1: pure-Rust builder).
//! Mirrors the layout of `templates/invoices/show.html`.
//!
//! PORTING INTO THE REAL APP (this file is the keeper):
//!   1. Move to `src/models/invoice/pdf.rs` and change the import below to
//!      `use super::{Invoice, InvoiceLineItem, grand_total, gst_total, subtotal};`
//!      — the real types are field-for-field compatible with what this uses.
//!   2. Replace `FONT_DIR` with a bundled `assets/fonts` dir (ship the TTFs in
//!      the repo), and load the family once at startup into `app_data` instead
//!      of per-call — see the integration plan.
//!   3. The `GET /invoices/{id}/pdf` handler calls `render_invoice_pdf(..)`.

use std::fs;

use genpdf::fonts::{FontData, FontFamily};
use genpdf::style::Style;
use genpdf::{Alignment, Document, Element, SimplePageDecorator, elements};
use rust_decimal::Decimal;

use crate::invoice::{Invoice, InvoiceLineItem, grand_total, gst_total, subtotal};

// Prototype: system fonts. In the real app this becomes the bundled assets dir.
const FONT_DIR: &str = "/usr/share/fonts/truetype/dejavu";

/// Real entry point: render an invoice to PDF bytes for the HTTP response body.
pub fn render_invoice_pdf(invoice: &Invoice, items: &[InvoiceLineItem]) -> Vec<u8> {
    let mut buf = Vec::new();
    build_invoice_pdf(invoice, items)
        .render(&mut buf)
        .expect("failed to render invoice PDF");
    buf
}

/// Format a decimal amount like the app's `money` Tera filter (`S$0.00`).
fn money(amount: Decimal) -> String {
    format!("S${amount:.2}")
}

/// Build the font family from the DejaVu fonts installed on the system.
///
/// DejaVu ships Regular + Bold here but no italic file, so the italic slots
/// reuse the upright faces (the invoice layout never renders italic anyway).
fn load_fonts() -> FontFamily<FontData> {
    let read =
        |name: &str| FontData::new(fs::read(format!("{FONT_DIR}/{name}")).unwrap(), None).unwrap();
    FontFamily {
        regular: read("DejaVuSans.ttf"),
        bold: read("DejaVuSans-Bold.ttf"),
        italic: read("DejaVuSans.ttf"),
        bold_italic: read("DejaVuSans-Bold.ttf"),
    }
}

fn bold() -> Style {
    Style::new().bold()
}

/// A "label: value" metadata line.
fn meta_line(label: &str, value: &str) -> elements::Paragraph {
    let mut p = elements::Paragraph::default();
    p.push_styled(format!("{label}: "), bold());
    p.push(value);
    p
}

/// Push a table row of cells, right-aligning everything after the description.
fn push_row(table: &mut elements::TableLayout, cells: [String; 7], header: bool) {
    let mut row = table.row();
    for (i, text) in cells.into_iter().enumerate() {
        let mut para = elements::Paragraph::new(text);
        if i > 0 {
            para.set_alignment(Alignment::Right);
        }
        let element = if header {
            para.styled(bold())
        } else {
            para.styled(Style::new())
        };
        row.push_element(element);
    }
    row.push().expect("row cell count matches column count");
}

/// Build the invoice PDF document, ready to render.
pub fn build_invoice_pdf(invoice: &Invoice, items: &[InvoiceLineItem]) -> Document {
    let mut doc = Document::new(load_fonts());
    doc.set_title(format!("Invoice {}", invoice.invoice_no));

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    // Header
    doc.push(
        elements::Paragraph::new("TAX INVOICE").styled(Style::new().bold().with_font_size(22)),
    );
    doc.push(elements::Break::new(1));

    // Invoice details
    doc.push(meta_line("Invoice No", &invoice.invoice_no));
    doc.push(meta_line("Status", &invoice.status.to_string()));
    doc.push(meta_line(
        "Party",
        invoice.party_name.as_deref().unwrap_or("—"),
    ));
    doc.push(meta_line("Issue Date", &invoice.issue_date.to_string()));
    doc.push(meta_line("Due Date", &invoice.due_date.to_string()));
    doc.push(elements::Break::new(1));

    // Line items table
    let mut table = elements::TableLayout::new(vec![5, 2, 3, 3, 3, 3, 3]);
    table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));

    push_row(
        &mut table,
        [
            "Description".into(),
            "Qty".into(),
            "Unit Price".into(),
            "GST Rate".into(),
            "Line Total".into(),
            "GST Amount".into(),
            "Total w/ GST".into(),
        ],
        true,
    );

    for item in items {
        push_row(
            &mut table,
            [
                item.description.clone(),
                item.quantity.to_string(),
                money(item.unit_price),
                item.gst_rate.to_string(),
                money(item.line_total),
                money(item.line_gst_amount),
                money(item.line_total_with_gst),
            ],
            false,
        );
    }

    doc.push(table);
    doc.push(elements::Break::new(1));

    // Totals summary, right-aligned
    let summary = |label: &str, value: Decimal, strong: bool| {
        let mut p = elements::Paragraph::default();
        let style = if strong { bold() } else { Style::new() };
        p.push_styled(format!("{label}: "), style);
        p.push_styled(money(value), style);
        p.set_alignment(Alignment::Right);
        p
    };
    doc.push(summary("Subtotal", subtotal(items), false));
    doc.push(summary("GST Total", gst_total(items), false));
    doc.push(summary("Grand Total", grand_total(items), true));

    doc
}
