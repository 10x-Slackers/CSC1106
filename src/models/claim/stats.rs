use rust_decimal::Decimal;
use sea_orm::sea_query::{Expr, Func, SimpleExpr};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QuerySelect};
use serde::Serialize;

use crate::entity::claim as claim_entity;
use crate::entity::claim::ClaimStatus;
use crate::models::error::ClaimError;

#[derive(Serialize)]
pub struct ClaimStats {
    pub pending_count: u64,
    pub pending_amount: Decimal,
    pub rejection_percentage: f64,
    pub avg_claim_amount: Decimal,
}

type StatsRow = (i64, Option<Decimal>, i64, i64, Option<Decimal>);

impl ClaimStats {
    pub async fn compute(db: &DatabaseConnection) -> Result<Self, ClaimError> {
        let result: Option<StatsRow> = claim_entity::Entity::find()
            .select_only()
            .column_as(
                SimpleExpr::from(Func::count(Expr::case(
                    Expr::col(claim_entity::Column::Status).eq(Expr::val(ClaimStatus::Pending)),
                    1,
                ))),
                "pending_count",
            )
            .column_as(
                SimpleExpr::from(Func::sum(
                    Expr::case(
                        Expr::col(claim_entity::Column::Status).eq(Expr::val(ClaimStatus::Pending)),
                        Expr::col(claim_entity::Column::Amount),
                    )
                    .finally(Expr::val(0)),
                )),
                "pending_amount",
            )
            .column_as(
                SimpleExpr::from(Func::count(Expr::case(
                    Expr::col(claim_entity::Column::Status).eq(Expr::val(ClaimStatus::Rejected)),
                    1,
                ))),
                "rejected_count",
            )
            .column_as(claim_entity::Column::Id.count(), "total_count")
            .column_as(
                SimpleExpr::from(Func::avg(Expr::col(claim_entity::Column::Amount))),
                "avg_amount",
            )
            .into_tuple()
            .one(db)
            .await?;

        let (pending_count, pending_amount, rejected_count, total_count, avg_amount) =
            result.unwrap_or((0, None, 0, 0, None));

        let rejection_percentage = if total_count == 0 {
            0.0
        } else {
            (rejected_count as f64 / total_count as f64) * 100.0
        };

        Ok(ClaimStats {
            pending_count: pending_count as u64,
            pending_amount: pending_amount.unwrap_or(Decimal::ZERO),
            rejection_percentage,
            avg_claim_amount: avg_amount.unwrap_or(Decimal::ZERO),
        })
    }
}
