//! Standalone prototype: render a hardcoded invoice to `sample.pdf` using
//! approach 1 (pure-Rust `genpdf` builder), reusing the project's invoice structs.

mod invoice;
mod pdf;

use chrono::NaiveDate;
use rust_decimal::dec;

use invoice::{GstRate, Invoice, InvoiceLineItem, InvoiceStatus};

fn main() {
    let items = vec![
        InvoiceLineItem::new(
            "Website design & build",
            1,
            dec!(4500.00),
            GstRate::Standard,
        ),
        InvoiceLineItem::new("Monthly hosting", 12, dec!(35.00), GstRate::Standard),
        InvoiceLineItem::new("Stock photography license", 3, dec!(120.00), GstRate::None),
        InvoiceLineItem::new(
            "On-site training (per day)",
            2,
            dec!(800.00),
            GstRate::Standard,
        ),
    ];

    let invoice = Invoice {
        invoice_no: "INV-2026-0001".into(),
        issue_date: NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(),
        due_date: NaiveDate::from_ymd_opt(2026, 7, 31).unwrap(),
        status: InvoiceStatus::Sent,
        party_name: Some("Acme Trading Pte Ltd".into()),
    };

    // Exercise the same entry point the real HTTP route will call.
    let bytes = pdf::render_invoice_pdf(&invoice, &items);
    std::fs::write("sample.pdf", bytes).expect("failed to write sample.pdf");

    println!("Wrote sample.pdf");
}
