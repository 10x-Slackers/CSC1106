use sea_orm::entity::prelude::*;

use super::user;

#[derive(Clone, Debug, DeriveEntityModel, Eq, PartialEq)]
#[sea_orm(table_name = "journal_entry")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub payment_id: Option<i32>,
    pub claim_id: Option<i32>,
    pub invoice_id: Option<i32>,
    pub created_by_user_id: i32,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedByUserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(has_many = "super::journal_entry_line::Entity")]
    JournalEntryLine,
}

impl Related<user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::journal_entry_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntryLine.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
