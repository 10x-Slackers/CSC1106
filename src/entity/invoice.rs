//! Invoice entity definition
//!
//!  Authors: Kitsuneez
use std::fmt;

use sea_orm::Iterable;
use sea_orm::Set;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Invoice status lifecycle.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum InvoiceStatus {
    #[sea_orm(string_value = "DRAFT")]
    Draft,
    #[sea_orm(string_value = "SENT")]
    Sent,
    #[sea_orm(string_value = "PARTIALLY_PAID")]
    PartiallyPaid,
    #[sea_orm(string_value = "PAID")]
    Paid,
    #[sea_orm(string_value = "VOIDED")]
    Voided,
}

impl fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvoiceStatus::Draft => write!(f, "Draft"),
            InvoiceStatus::Sent => write!(f, "Sent"),
            InvoiceStatus::PartiallyPaid => write!(f, "Partially Paid"),
            InvoiceStatus::Paid => write!(f, "Paid"),
            InvoiceStatus::Voided => write!(f, "Voided"),
        }
    }
}

impl InvoiceStatus {
    /// Return all status variant labels in declaration order.
    pub fn labels() -> Vec<String> {
        Self::iter().map(|v| v.to_string()).collect()
    }

    pub fn parse(s: &str) -> Option<Self> {
        let key = s.trim().to_lowercase().replace(' ', "");
        match key.as_str() {
            "draft" => Some(InvoiceStatus::Draft),
            "sent" => Some(InvoiceStatus::Sent),
            "partiallypaid" => Some(InvoiceStatus::PartiallyPaid),
            "paid" => Some(InvoiceStatus::Paid),
            "voided" => Some(InvoiceStatus::Voided),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, DeriveEntityModel, Eq, PartialEq)]
#[sea_orm(table_name = "invoice")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub party_id: i32,
    pub created_by_user_id: i32,
    #[sea_orm(unique)]
    pub invoice_no: String,
    pub issue_date: Date,
    pub due_date: Date,
    pub status: InvoiceStatus,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
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
    #[sea_orm(has_many = "super::invoice_line_item::Entity")]
    InvoiceLineItem,
    #[sea_orm(has_many = "super::payment::Entity")]
    Payment,
    #[sea_orm(has_many = "super::journal_entry::Entity")]
    JournalEntry,
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

impl Related<super::invoice_line_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InvoiceLineItem.def()
    }
}

impl Related<super::payment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Payment.def()
    }
}

impl Related<super::journal_entry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntry.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        let now = chrono::Utc::now().naive_utc();
        let mut am = self;
        if insert && am.is_not_set(Column::CreatedAt) {
            am.created_at = Set(now);
        }
        am.updated_at = Set(now);
        Ok(am)
    }
}
