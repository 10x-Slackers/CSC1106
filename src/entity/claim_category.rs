use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, DeriveEntityModel, Eq, PartialEq)]
#[sea_orm(table_name = "claim_category")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::claim::Entity")]
    Claim,
}

impl Related<super::claim::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Claim.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
