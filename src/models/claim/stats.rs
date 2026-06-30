use rust_decimal::Decimal;
use sea_orm::sea_query::{Expr, Func, SimpleExpr};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};
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

impl ClaimStats {
    pub async fn compute(db: &DatabaseConnection) -> Result<Self, ClaimError> {
        let result: Option<(i64, Option<Decimal>)> = claim_entity::Entity::find()
            .select_only()
            .column_as(claim_entity::Column::Id.count(), "cnt")
            .column_as(Expr::col(claim_entity::Column::Amount).sum(), "sum_amount")
            .filter(claim_entity::Column::Status.eq(ClaimStatus::Pending))
            .into_tuple()
            .one(db)
            .await?;
        let (pending_count, pending_amount) = result.unwrap_or((0, None));

        let result: Option<(i64,)> = claim_entity::Entity::find()
            .select_only()
            .column_as(claim_entity::Column::Id.count(), "cnt")
            .filter(claim_entity::Column::Status.eq(ClaimStatus::Rejected))
            .into_tuple()
            .one(db)
            .await?;
        let rejected_count = result.unwrap_or((0,)).0;

        let result: Option<(i64, Option<Decimal>)> = claim_entity::Entity::find()
            .select_only()
            .column_as(claim_entity::Column::Id.count(), "cnt")
            .column_as(
                SimpleExpr::from(Func::avg(Expr::col(claim_entity::Column::Amount))),
                "avg_amount",
            )
            .into_tuple()
            .one(db)
            .await?;
        let (total_count, avg_amount) = result.unwrap_or((0, None));

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
