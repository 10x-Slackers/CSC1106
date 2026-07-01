use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::entity::invoice as invoice_entity;
use crate::entity::invoice::InvoiceStatus;
use crate::entity::invoice_line_item as line_item_entity;
use crate::entity::invoice_line_item::GstRate;

use super::calc::{line_gst_amount, line_total};

/// Application-level invoice model with enrichment fields for rendering.
#[derive(Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: i32,
    pub party_id: i32,
    pub created_by_user_id: i32,
    pub invoice_no: String,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    pub status: InvoiceStatus,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub party_name: Option<String>,
    pub created_by_name: Option<String>,
    #[serde(skip_deserializing)]
    pub grand_total: Decimal,
}

impl From<invoice_entity::Model> for Invoice {
    fn from(m: invoice_entity::Model) -> Self {
        Invoice {
            id: m.id,
            party_id: m.party_id,
            created_by_user_id: m.created_by_user_id,
            invoice_no: m.invoice_no,
            issue_date: m.issue_date,
            due_date: m.due_date,
            status: m.status,
            created_at: m.created_at,
            updated_at: m.updated_at,
            party_name: None,
            created_by_name: None,
            grand_total: Decimal::ZERO,
        }
    }
}

/// Invoice line item with precomputed totals for display.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvoiceLineItem {
    pub invoice_id: i32,
    pub description: String,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub gst_rate: GstRate,
    pub line_total: Decimal,
    pub line_gst_amount: Decimal,
    pub line_total_with_gst: Decimal,
}

impl From<line_item_entity::Model> for InvoiceLineItem {
    fn from(m: line_item_entity::Model) -> Self {
        let total = line_total(m.quantity, m.unit_price);
        let gst = line_gst_amount(m.quantity, m.unit_price, &m.gst_rate);
        InvoiceLineItem {
            invoice_id: m.invoice_id,
            description: m.description,
            quantity: m.quantity,
            unit_price: m.unit_price,
            gst_rate: m.gst_rate,
            line_total: total,
            line_gst_amount: gst,
            line_total_with_gst: total + gst,
        }
    }
}

/// Input for a single line item when creating or updating an invoice.
#[derive(Clone, Debug)]
pub struct LineItemInput {
    pub description: String,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub gst_rate: GstRate,
}

/// Group of line items for a single invoice.
#[derive(Clone, Serialize)]
pub struct GstInvoiceLineGroup {
    pub invoice_id: i32,
    pub invoice_no: String,
    pub issue_date: NaiveDate,
    pub items: Vec<InvoiceLineItem>,
}
