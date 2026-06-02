use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, ActiveValue::Set, FromQueryResult};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Eq)]
#[sea_orm(table_name = "invoice")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub customer_id: i32,
    pub created_by: i32,
    pub invoice_number: String,
    pub issue_date: i64,
    pub due_date: i64,
    pub reference: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((15, 2)))")]
    pub subtotal: Decimal,
    #[sea_orm(column_type = "Decimal(Some((15, 2)))")]
    pub total_amount: Decimal,
    #[sea_orm(column_type = "Decimal(Some((15, 2)))")]
    pub paid_amount: Decimal,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    CreatedBy,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedBy.def()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, FromQueryResult)]
pub struct InvoiceRow {
    pub id: i32,
    pub customer_id: i32,
    pub created_by: i32,
    pub invoice_number: String,
    pub issue_date: i64,
    pub due_date: i64,
    pub reference: Option<String>,
    pub subtotal: Decimal,
    pub total_amount: Decimal,
    pub paid_amount: Decimal,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub customer_name: String,
    pub created_by_name: String,
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
