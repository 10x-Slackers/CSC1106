use std::fmt;

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
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
