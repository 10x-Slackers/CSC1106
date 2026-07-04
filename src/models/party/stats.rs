//! Computes summary statistics for parties.
//!
//! Authors: Tan Yong Meng

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use serde::Serialize;

use crate::entity::party as party_entity;
use crate::entity::party::{PartyStatus, PartyType};
use crate::models::error::AppError;

/// Summary statistics for party records.
#[derive(Serialize)]
pub struct PartyStats {
    pub total: u64,
    pub active_customers: u64,
    pub active_vendors: u64,
}

impl PartyStats {
    /// Computes party statistics from the database.
    ///
    /// Counts the total number of parties, active customers, and active vendors.
    /// The count queries are executed concurrently.
    pub async fn compute(db: &DatabaseConnection) -> Result<Self, AppError> {
        let total_fut = party_entity::Entity::find().count(db);
        let customers_fut = party_entity::Entity::find()
            .filter(party_entity::Column::PartyType.eq(PartyType::Customer))
            .filter(party_entity::Column::Status.eq(PartyStatus::Active))
            .count(db);
        let vendors_fut = party_entity::Entity::find()
            .filter(party_entity::Column::PartyType.eq(PartyType::Vendor))
            .filter(party_entity::Column::Status.eq(PartyStatus::Active))
            .count(db);

        let (total, active_customers, active_vendors) =
            futures::try_join!(total_fut, customers_fut, vendors_fut)?;

        Ok(PartyStats {
            total,
            active_customers,
            active_vendors,
        })
    }
}
