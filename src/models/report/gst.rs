use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;

use crate::entity::invoice_line_item::GstRate;
use crate::models::error::AppError;
use crate::models::invoice::Invoice;

#[derive(serde::Serialize)]
pub struct GstInvoice {
    pub invoice_id: i32,
    pub invoice_no: String,
    pub issue_date: chrono::NaiveDate,
    pub gst_amount: Decimal,
}

#[derive(serde::Serialize)]
pub struct GstSummary {
    pub from: Option<chrono::NaiveDate>,
    pub to: Option<chrono::NaiveDate>,
    pub total_gst: Decimal,
    pub invoices: Vec<GstInvoice>,
}

pub struct GstReport;

impl GstReport {
    /// Only counts GST charged on sales invoices. Doesn't subtract GST paid on purchases.
    pub async fn compute(
        db: &DatabaseConnection,
        from: Option<chrono::NaiveDate>,
        to: Option<chrono::NaiveDate>,
    ) -> Result<GstSummary, AppError> {
        let groups = Invoice::line_items_for_period(db, from, to).await?;

        let mut invoices = Vec::new();
        let mut total_gst = Decimal::ZERO;

        for group in groups {
            let gst_amount: Decimal = group
                .items
                .iter()
                .filter(|item| item.gst_rate == GstRate::Standard)
                .map(|item| item.line_gst_amount)
                .sum();

            if gst_amount == Decimal::ZERO {
                continue;
            }

            total_gst += gst_amount;
            invoices.push(GstInvoice {
                invoice_id: group.invoice_id,
                invoice_no: group.invoice_no,
                issue_date: group.issue_date,
                gst_amount,
            });
        }

        Ok(GstSummary {
            from,
            to,
            total_gst,
            invoices,
        })
    }
}
