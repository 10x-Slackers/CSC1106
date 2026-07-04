//! Defines the SeaORM database entity for claim categories
//!
//! Authors: Teo Kai Wen

use sea_orm::entity::prelude::*;

/// Represents a claim category record in the `claim_category` database table.
///
/// Claim categories are used to group claims by type, such as transport,
/// meals, office supplies, or other expense categories.
#[derive(Clone, Debug, DeriveEntityModel, Eq, PartialEq)]
#[sea_orm(table_name = "claim_category")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
}

/// Defines relationships between the `claim_category` table and other tables.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// A claim category can be associated with many claims.
    #[sea_orm(has_many = "super::claim::Entity")]
    Claim,
}

impl Related<super::claim::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Claim.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
