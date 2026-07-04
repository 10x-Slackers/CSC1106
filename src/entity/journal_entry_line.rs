//! One debit or credit row within a journal entry.
//!
//! Authors: Felicia

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::account;
use super::journal_entry;

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(10))")]
pub enum EntrySide {
    #[sea_orm(string_value = "DEBIT")]
    Debit,
    #[sea_orm(string_value = "CREDIT")]
    Credit,
}

impl fmt::Display for EntrySide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntrySide::Debit => write!(f, "Debit"),
            EntrySide::Credit => write!(f, "Credit"),
        }
    }
}

/// `amount` is always positive. Whether a line is a debit or credit is
/// decided by `entry_side`. This rule is enforced in code 
/// (`JournalEntryLineInput::validate_all`).
#[derive(Clone, Debug, DeriveEntityModel, Eq, PartialEq)]
#[sea_orm(table_name = "journal_entry_line")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub entry_id: i32,
    pub account_id: i32,
    pub entry_side: EntrySide,
    #[sea_orm(column_type = "Decimal(Some((15, 4)))")]
    pub amount: Decimal,
    pub description: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::journal_entry::Entity",
        from = "Column::EntryId",
        to = "super::journal_entry::Column::Id"
    )]
    JournalEntry,
    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::AccountId",
        to = "super::account::Column::Id"
    )]
    Account,
}

impl Related<journal_entry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntry.def()
    }
}

impl Related<account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
