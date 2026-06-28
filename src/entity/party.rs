use std::fmt;

use sea_orm::Iterable;
use sea_orm::Set;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Party type: customer or vendor.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum PartyType {
    #[sea_orm(string_value = "CUSTOMER")]
    Customer,
    #[sea_orm(string_value = "VENDOR")]
    Vendor,
}

impl fmt::Display for PartyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartyType::Customer => write!(f, "Customer"),
            PartyType::Vendor => write!(f, "Vendor"),
        }
    }
}

impl PartyType {
    /// Return all party type variant labels in declaration order.
    pub fn labels() -> Vec<String> {
        Self::iter().map(|v| v.to_string()).collect()
    }
}

/// Party status: active or inactive.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum PartyStatus {
    #[sea_orm(string_value = "ACTIVE")]
    Active,
    #[sea_orm(string_value = "INACTIVE")]
    Inactive,
}

impl fmt::Display for PartyStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartyStatus::Active => write!(f, "Active"),
            PartyStatus::Inactive => write!(f, "Inactive"),
        }
    }
}

impl PartyStatus {
    /// Return all status variant labels in declaration order.
    pub fn labels() -> Vec<String> {
        Self::iter().map(|v| v.to_string()).collect()
    }
}

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

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::invoice::Entity")]
    Invoice,
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
