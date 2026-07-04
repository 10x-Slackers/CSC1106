//! Defines the SeaORM database entity for users in the system.
//!
//! Authors: Tan Yong Meng

use sea_orm::Iterable;
use sea_orm::Set;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum Role {
    #[sea_orm(string_value = "STAFF")]
    Staff,
    #[sea_orm(string_value = "ACCOUNTANT")]
    Accountant,
    #[sea_orm(string_value = "ADMIN")]
    Admin,
}

impl fmt::Display for Role {
    /// Formats the role as a label.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Staff => write!(f, "Staff"),
            Role::Accountant => write!(f, "Accountant"),
            Role::Admin => write!(f, "Admin"),
        }
    }
}

impl Role {
    /// Parse a string into a [`Role`].
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "staff" => Some(Role::Staff),
            "accountant" => Some(Role::Accountant),
            "admin" => Some(Role::Admin),
            _ => None,
        }
    }

    /// Return all role labels.
    pub fn labels() -> Vec<String> {
        Self::iter().map(|v| v.to_string()).collect()
    }
}

/// Represents the account status of a user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserStatus {
    Active,
    Disabled,
}

impl UserStatus {
    /// Parse a string into a [`UserStatus`].
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "active" => Some(UserStatus::Active),
            "disabled" => Some(UserStatus::Disabled),
            _ => None,
        }
    }

    /// Returns whether the user status is disabled
    pub fn disabled(&self) -> bool {
        match self {
            UserStatus::Active => false,
            UserStatus::Disabled => true,
        }
    }
}

/// Represents a user record in the `user` database
#[derive(Clone, Debug, DeriveEntityModel, Eq, PartialEq)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub role: Role,
    pub disabled: bool,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

/// Defines relationships between the user and other database tables
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::invoice::Entity")]
    Invoice,
    #[sea_orm(has_many = "super::payment::Entity")]
    Payment,
    #[sea_orm(has_many = "super::journal_entry::Entity")]
    JournalEntry,
}

impl Related<super::claim::Entity> for Entity {
    fn to() -> RelationDef {
        super::claim::Relation::SubmittedByUser.def()
    }

    fn via() -> Option<RelationDef> {
        None
    }
}

impl Related<super::invoice::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invoice.def()
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
    /// Runs automatically when a record is inserted or updated.
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        let now = chrono::Utc::now().naive_utc();
        let mut am = self;
        // If creation time is not set, it will set here
        if insert && am.is_not_set(Column::CreatedAt) {
            am.created_at = Set(now);
        }
        // Updates the `updated_at` every time the record is saved.
        am.updated_at = Set(now);
        Ok(am)
    }
}
