//! Defines the SeaORM database entity for parties in the accounting system.
//!
//! Authors: Tan Yong Meng

use sea_orm::Iterable;
use sea_orm::Set;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the different types of parties in the accounting system.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum PartyType {
    #[sea_orm(string_value = "CUSTOMER")]
    Customer,
    #[sea_orm(string_value = "VENDOR")]
    Vendor,
}

impl fmt::Display for PartyType {
    /// Formats the different party type to a label.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartyType::Customer => write!(f, "Customer"),
            PartyType::Vendor => write!(f, "Vendor"),
        }
    }
}

impl PartyType {
    /// Returns all available party types as strings.
    pub fn labels() -> Vec<String> {
        Self::iter().map(|v| v.to_string()).collect()
    }
}

/// Used to determine whether a party is active or inactive
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum PartyStatus {
    #[sea_orm(string_value = "ACTIVE")]
    Active,
    #[sea_orm(string_value = "INACTIVE")]
    Inactive,
}

impl fmt::Display for PartyStatus {
    /// Formats the different status to a label
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartyStatus::Active => write!(f, "Active"),
            PartyStatus::Inactive => write!(f, "Inactive"),
        }
    }
}

impl PartyStatus {
    /// Return all available party status (Active/Inactive).
    pub fn labels() -> Vec<String> {
        Self::iter().map(|v| v.to_string()).collect()
    }
}

/// Represents a party record in the database
#[derive(Clone, Debug, DeriveEntityModel, Eq, PartialEq)]
#[sea_orm(table_name = "party")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub party_type: PartyType,
    pub name: String,
    pub company: Option<String>,
    #[sea_orm(unique)]
    pub email: String,
    pub phone: String,
    pub address: String,
    pub status: PartyStatus,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

/// Defines relationships from a party to its invoices and payments.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// A party can have many invoices
    #[sea_orm(has_many = "super::invoice::Entity")]
    Invoice,
    /// A party can have many payments
    #[sea_orm(has_many = "super::payment::Entity")]
    Payment,
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
