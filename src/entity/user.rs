use std::fmt;

use sea_orm::Set;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// User role controlling access level.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum Role {
    #[sea_orm(string_value = "ADMIN")]
    Admin,
    #[sea_orm(string_value = "ACCOUNTANT")]
    Accountant,
    #[sea_orm(string_value = "STAFF")]
    Staff,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Admin => write!(f, "Admin"),
            Role::Accountant => write!(f, "Accountant"),
            Role::Staff => write!(f, "Staff"),
        }
    }
}

impl Role {
    /// Parse a role string (case-insensitive) into a `Role`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "admin" => Some(Role::Admin),
            "accountant" => Some(Role::Accountant),
            "staff" => Some(Role::Staff),
            _ => None,
        }
    }
}

/// Account status filter, mapping to the `disabled` boolean column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserStatus {
    Active,
    Disabled,
}

impl UserStatus {
    /// Parse a status string (case-insensitive) into a `UserStatus`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "active" => Some(UserStatus::Active),
            "disabled" => Some(UserStatus::Disabled),
            _ => None,
        }
    }

    /// The `disabled` column value this status maps to.
    pub fn disabled(&self) -> bool {
        match self {
            UserStatus::Active => false,
            UserStatus::Disabled => true,
        }
    }
}

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

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::invoice::Entity")]
    Invoice,
    #[sea_orm(has_many = "super::payment::Entity")]
    Payment,
    #[sea_orm(has_many = "super::journal_entry::Entity")]
    JournalEntry,
}

/// Related<claim::Entity> only resolves submitter relation.
/// To find claims reviewed by this user, use with `claim::Relation::ReviewedByUser.def()` directly.
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
