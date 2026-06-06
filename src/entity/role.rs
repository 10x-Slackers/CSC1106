use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

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
