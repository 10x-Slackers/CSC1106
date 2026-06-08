use std::fmt;

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Payment direction: money in or out.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(10))")]
pub enum PaymentDirection {
    #[sea_orm(string_value = "IN")]
    In,
    #[sea_orm(string_value = "OUT")]
    Out,
}

impl fmt::Display for PaymentDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaymentDirection::In => write!(f, "In"),
            PaymentDirection::Out => write!(f, "Out"),
        }
    }
}

#[derive(Clone, Debug, DeriveEntityModel, Eq, PartialEq)]
#[sea_orm(table_name = "payment")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub invoice_id: Option<i32>,
    pub party_id: Option<i32>,
    pub created_by_user_id: i32,
    pub payment_direction: PaymentDirection,
    #[sea_orm(column_type = "Decimal(Some((15, 4)))")]
    pub amount: Decimal,
    pub payment_date: Date,
    pub remarks: Option<String>,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::invoice::Entity",
        from = "Column::InvoiceId",
        to = "super::invoice::Column::Id"
    )]
    Invoice,
    #[sea_orm(
        belongs_to = "super::party::Entity",
        from = "Column::PartyId",
        to = "super::party::Column::Id"
    )]
    Party,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedByUserId",
        to = "super::user::Column::Id"
    )]
    CreatedByUser,
    #[sea_orm(has_many = "super::journal_entry::Entity")]
    JournalEntry,
}

impl Related<super::invoice::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invoice.def()
    }
}

impl Related<super::party::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Party.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedByUser.def()
    }
}

impl Related<super::journal_entry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntry.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
