use sea_orm::sea_query::{Expr, Func, SimpleExpr};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QuerySelect};
use serde::Serialize;

use crate::entity::invoice as invoice_entity;
use crate::entity::invoice::InvoiceStatus;
use crate::models::error::AppError;

#[derive(Serialize)]
pub struct InvoiceStats {
    pub total: u64,
    pub draft: u64,
    pub unpaid: u64,
}

impl InvoiceStats {
    /// Compute the invoice total, draft, and unpaid counts from the database.
    pub async fn compute(db: &DatabaseConnection) -> Result<Self, AppError> {
        let result: Option<(i64, i64, i64)> = invoice_entity::Entity::find()
            .select_only()
            .column_as(invoice_entity::Column::Id.count(), "total")
            .column_as(
                SimpleExpr::from(Func::count(Expr::case(
                    Expr::col(invoice_entity::Column::Status).eq(Expr::val(InvoiceStatus::Draft)),
                    1,
                ))),
                "draft",
            )
            .column_as(
                SimpleExpr::from(Func::count(Expr::case(
                    Expr::col(invoice_entity::Column::Status)
                        .is_in([InvoiceStatus::Sent, InvoiceStatus::PartiallyPaid]),
                    1,
                ))),
                "unpaid",
            )
            .into_tuple()
            .one(db)
            .await?;

        let (total, draft, unpaid) = result.unwrap_or((0, 0, 0));

        Ok(InvoiceStats {
            total: total as u64,
            draft: draft as u64,
            unpaid: unpaid as u64,
        })
    }
}
