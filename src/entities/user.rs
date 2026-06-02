use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, ActiveValue::Set};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum Role {
    #[sea_orm(string_value = "ADMIN")]
    ADMIN,
    #[sea_orm(string_value = "ACCOUNTANT")]
    ACCOUNTANT,
    #[sea_orm(string_value = "STAFF")]
    STAFF,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub role: Role,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::invoice::Entity")]
    CreatedInvoice,
}

impl Related<super::invoice::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedInvoice.def()
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
