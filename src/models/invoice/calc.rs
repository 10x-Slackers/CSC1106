use rust_decimal::Decimal;

use super::types::InvoiceLineItem;
use crate::entity::invoice_line_item::GstRate;

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
    items
        .iter()
        .map(|i| line_total(i.quantity, i.unit_price))
        .sum()
}

/// Sum of GST amounts across all lines.
pub fn gst_total(items: &[InvoiceLineItem]) -> Decimal {
    items
        .iter()
        .map(|i| line_gst_amount(i.quantity, i.unit_price, &i.gst_rate))
        .sum()
}

/// Subtotal plus GST total.
pub fn grand_total(items: &[InvoiceLineItem]) -> Decimal {
    subtotal(items) + gst_total(items)
}
