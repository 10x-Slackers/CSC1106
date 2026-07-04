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
    /// Used to convert the enum into a string to display in a dropdown/form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartyType::Customer => write!(f, "Customer"),
            PartyType::Vendor => write!(f, "Vendor"),
        }
    }
}

impl PartyType {
    /// Returns all available party types as strings.
    /// Used to later generate dropdown options in the frontend UI.
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
    /// Used to convert the enum into a string to display in a dropdown/form
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
///
/// A party can be either a customer or vendor, storing basic information.
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
    /// Defines the relationship from party to invoice.
    ///
    /// Tells SeaORM that a party can be related to many invoice records.
    fn to() -> RelationDef {
        Relation::Invoice.def()
    }
}

impl Related<super::payment::Entity> for Entity {
    /// Defines the relationship from party to payment.
    ///
    /// Tells SeaORM that a party can be related to many payment records.
    fn to() -> RelationDef {
        Relation::Payment.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// Runs automatically when a record is inserted or updated.
    ///
    /// When inserting a new record, this function sets `created_at` if it has
    /// not already been set. It also updates `updated_at` every time the record
    /// is saved.
    ///
    /// Returns the modified record with updated timestamp fields.
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
