//! Invoice domain types reused from the main project (DB/serde coupling stripped
//! for this standalone prototype).

use std::fmt;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal::dec;

/// GST rate applied to an invoice line item.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GstRate {
    None,
    Standard,
}

impl fmt::Display for GstRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GstRate::None => write!(f, "None (0%)"),
            GstRate::Standard => write!(f, "Standard (9%)"),
        }
    }
}

impl GstRate {
    /// Singapore GST standard rate (9%).
    pub const STANDARD_RATE: Decimal = dec!(0.09);

    pub fn rate(&self) -> Decimal {
        match self {
            GstRate::None => Decimal::ZERO,
            GstRate::Standard => Self::STANDARD_RATE,
        }
    }
}

/// Invoice status lifecycle.
// Prototype only constructs `Sent`; the real app (SeaORM enum) uses every
// variant, so this `allow` is prototype-only and won't be ported.
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InvoiceStatus {
    Draft,
    Sent,
    PartiallyPaid,
    Paid,
    Voided,
}

impl fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvoiceStatus::Draft => write!(f, "Draft"),
            InvoiceStatus::Sent => write!(f, "Sent"),
            InvoiceStatus::PartiallyPaid => write!(f, "Partially Paid"),
            InvoiceStatus::Paid => write!(f, "Paid"),
            InvoiceStatus::Voided => write!(f, "Voided"),
        }
    }
}

/// Invoice line item with precomputed totals for display.
#[derive(Clone)]
pub struct InvoiceLineItem {
    pub description: String,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub gst_rate: GstRate,
    pub line_total: Decimal,
    pub line_gst_amount: Decimal,
    pub line_total_with_gst: Decimal,
}

impl InvoiceLineItem {
    /// Build a line item, computing its totals the same way the main app does.
    pub fn new(description: &str, quantity: i32, unit_price: Decimal, gst_rate: GstRate) -> Self {
        let total = line_total(quantity, unit_price);
        let gst = line_gst_amount(quantity, unit_price, &gst_rate);
        InvoiceLineItem {
            description: description.to_string(),
            quantity,
            unit_price,
            gst_rate,
            line_total: total,
            line_gst_amount: gst,
            line_total_with_gst: total + gst,
        }
    }
}

/// Application-level invoice model.
#[derive(Clone)]
pub struct Invoice {
    pub invoice_no: String,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    pub status: InvoiceStatus,
    pub party_name: Option<String>,
}

// --- calc helpers (mirrors src/models/invoice/calc.rs) ---

/// Line total before GST.
pub fn line_total(quantity: i32, unit_price: Decimal) -> Decimal {
    Decimal::from(quantity) * unit_price
}

/// GST amount for a line rounded to 4 decimal places.
pub fn line_gst_amount(quantity: i32, unit_price: Decimal, gst_rate: &GstRate) -> Decimal {
    (line_total(quantity, unit_price) * gst_rate.rate()).round_dp(4)
}

/// Sum of line totals before GST.
pub fn subtotal(items: &[InvoiceLineItem]) -> Decimal {
    items.iter().map(|i| i.line_total).sum()
}

/// Sum of GST amounts across all lines.
pub fn gst_total(items: &[InvoiceLineItem]) -> Decimal {
    items.iter().map(|i| i.line_gst_amount).sum()
}

/// Subtotal plus GST total.
pub fn grand_total(items: &[InvoiceLineItem]) -> Decimal {
    subtotal(items) + gst_total(items)
}
