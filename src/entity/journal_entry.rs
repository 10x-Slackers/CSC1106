//! The header row for a posted double-entry transaction.
//!
//! Authors: commit2main

use sea_orm::Set;
use sea_orm::entity::prelude::*;

/// Only one of `payment_id`/`claim_id`/`invoice_id` should be set at a time.
#[derive(Clone, Debug, DeriveEntityModel, Eq, PartialEq)]
#[sea_orm(table_name = "journal_entry")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub payment_id: Option<i32>,
    pub claim_id: Option<i32>,
    pub invoice_id: Option<i32>,
    pub created_by_user_id: i32,
    pub created_at: DateTime,
}

/// Each journal entry traces back to only one source document.
#[derive(Clone)]
pub enum SourceDocument {
    Payment { payment_id: i32 },
    Claim { claim_id: i32 },
    Invoice { invoice_id: i32 },
}

impl SourceDocument {
    /// Map the source document into the three optional id columns it can set.
    pub fn to_fks(&self) -> (Option<i32>, Option<i32>, Option<i32>) {
        match self {
            SourceDocument::Payment { payment_id } => (Some(*payment_id), None, None),
            SourceDocument::Claim { claim_id } => (None, Some(*claim_id), None),
            SourceDocument::Invoice { invoice_id } => (None, None, Some(*invoice_id)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedByUserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::payment::Entity",
        from = "Column::PaymentId",
        to = "super::payment::Column::Id"
    )]
    Payment,
    #[sea_orm(
        belongs_to = "super::claim::Entity",
        from = "Column::ClaimId",
        to = "super::claim::Column::Id"
    )]
    Claim,
    #[sea_orm(
        belongs_to = "super::invoice::Entity",
        from = "Column::InvoiceId",
        to = "super::invoice::Column::Id"
    )]
    Invoice,
    #[sea_orm(has_many = "super::journal_entry_line::Entity")]
    JournalEntryLine,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::journal_entry_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntryLine.def()
    }
}

impl Related<super::payment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Payment.def()
    }
}

impl Related<super::claim::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Claim.def()
    }
}

impl Related<super::invoice::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invoice.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        let mut am = self;
        if insert && am.is_not_set(Column::CreatedAt) {
            am.created_at = Set(chrono::Utc::now().naive_utc());
        }
        Ok(am)
    }
}
