use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, ActiveValue::Set};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum CustomerType {
    #[sea_orm(string_value = "CUSTOMER")]
    CUSTOMER,
    #[sea_orm(string_value = "VENDOR")]
    VENDOR,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum CustomerStatus {
    #[sea_orm(string_value = "ACTIVE")]
    ACTIVE,
    #[sea_orm(string_value = "ARCHIVED")]
    ARCHIVED,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize)]
#[sea_orm(table_name = "customer")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub type_: CustomerType,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub company: Option<String>,
    pub status: CustomerStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::invoice::Entity")]
    Invoice,
}

impl Related<super::invoice::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invoice.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            created_at: Set(chrono::Utc::now().timestamp()),
            updated_at: Set(chrono::Utc::now().timestamp()),
            ..<Self as ActiveModelTrait>::default()
        }
    }
}
