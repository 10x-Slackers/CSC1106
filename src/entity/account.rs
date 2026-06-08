use std::fmt;

use sea_orm::Set;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, DeriveEntityModel, Eq, PartialEq)]
#[sea_orm(table_name = "account")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
    pub category: AccountCategory,
    pub normal_balance: NormalBalance,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::journal_entry_line::Entity")]
    JournalEntryLine,
}

impl Related<super::journal_entry_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntryLine.def()
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

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum AccountCategory {
    #[sea_orm(string_value = "ASSET")]
    Asset,
    #[sea_orm(string_value = "LIABILITY")]
    Liability,
    #[sea_orm(string_value = "EQUITY")]
    Equity,
    #[sea_orm(string_value = "REVENUE")]
    Revenue,
    #[sea_orm(string_value = "EXPENSE")]
    Expense,
}

impl fmt::Display for AccountCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountCategory::Asset => write!(f, "Asset"),
            AccountCategory::Liability => write!(f, "Liability"),
            AccountCategory::Equity => write!(f, "Equity"),
            AccountCategory::Revenue => write!(f, "Revenue"),
            AccountCategory::Expense => write!(f, "Expense"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(10))")]
pub enum NormalBalance {
    #[sea_orm(string_value = "DEBIT")]
    Debit,
    #[sea_orm(string_value = "CREDIT")]
    Credit,
}

impl fmt::Display for NormalBalance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NormalBalance::Debit => write!(f, "Debit"),
            NormalBalance::Credit => write!(f, "Credit"),
        }
    }
}
