use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use serde::Serialize;

use crate::entity::user as user_entity;
use crate::entity::user::Role;
use crate::models::error::AppError;

/// Summary statistics for active users by role.
#[derive(Serialize)]
pub struct UserStats {
    pub admin: u64,
    pub accountant: u64,
    pub staff: u64,
}

impl UserStats {
    /// Computes user statistics from the database.
    ///
    /// Counts active users grouped by role. Disabled users are excluded from
    /// all counts. The role count queries are executed concurrently.
    pub async fn compute(db: &DatabaseConnection) -> Result<Self, AppError> {
        let admin_fut = user_entity::Entity::find()
            .filter(user_entity::Column::Role.eq(Role::Admin))
            .filter(user_entity::Column::Disabled.eq(false))
            .count(db);
        let finance_fut = user_entity::Entity::find()
            .filter(user_entity::Column::Role.eq(Role::Accountant))
            .filter(user_entity::Column::Disabled.eq(false))
            .count(db);
        let staff_fut = user_entity::Entity::find()
            .filter(user_entity::Column::Role.eq(Role::Staff))
            .filter(user_entity::Column::Disabled.eq(false))
            .count(db);

        let (admin, accountant, staff) = futures::try_join!(admin_fut, finance_fut, staff_fut)?;

        Ok(UserStats {
            admin,
            accountant,
            staff,
        })
    }
}
