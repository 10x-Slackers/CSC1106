use std::fmt;

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Claim status.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum ClaimStatus {
    #[sea_orm(string_value = "PENDING")]
    Pending,
    #[sea_orm(string_value = "APPROVED")]
    Approved,
    #[sea_orm(string_value = "REJECTED")]
    Rejected,
}

impl fmt::Display for ClaimStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClaimStatus::Pending => write!(f, "Pending"),
            ClaimStatus::Approved => write!(f, "Approved"),
            ClaimStatus::Rejected => write!(f, "Rejected"),
        }
    }
}

#[derive(Clone, Debug, DeriveEntityModel, Eq, PartialEq)]
#[sea_orm(table_name = "claim")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub submitted_by_user_id: i32,
    pub reviewed_by_user_id: Option<i32>,
    pub category_id: i32,
    pub title: String,
    pub description: String,
    #[sea_orm(column_type = "Decimal(Some((15, 4)))")]
    pub amount: Decimal,
    pub purchase_date: Date,
    pub status: ClaimStatus,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::SubmittedByUserId",
        to = "super::user::Column::Id"
    )]
    SubmittedByUser,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ReviewedByUserId",
        to = "super::user::Column::Id"
    )]
    ReviewedByUser,
    #[sea_orm(
        belongs_to = "super::claim_category::Entity",
        from = "Column::CategoryId",
        to = "super::claim_category::Column::Id"
    )]
    ClaimCategory,
    #[sea_orm(has_many = "super::journal_entry::Entity")]
    JournalEntry,
}

/// `Related<user::Entity>` only resolves the submitter relation.
/// To load the reviewer, use `Relation::ReviewedByUser.def()` explicitly.
impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SubmittedByUser.def()
    }
}

impl Related<super::claim_category::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ClaimCategory.def()
    }
}

impl Related<super::journal_entry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntry.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
