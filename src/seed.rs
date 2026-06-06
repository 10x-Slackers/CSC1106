use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::entity::role::Role;
use crate::entity::user;
use crate::models::user::create_user;

pub async fn seed(db: &DatabaseConnection) {
    // Check if already seeded
    if user::Entity::find()
        .filter(user::Column::Email.eq("admin@example.com"))
        .one(db)
        .await
        .expect("DB query failed")
        .is_some()
    {
        return;
    }

    // Create users
    create_user(
        db,
        "admin@example.com",
        "Administrator",
        "P@ssw0rd",
        Role::Admin,
    )
    .await
    .expect("Failed to seed admin user");

    create_user(
        db,
        "john@example.com",
        "John Doe",
        "P@ssw0rd",
        Role::Accountant,
    )
    .await
    .expect("Failed to seed accountant user");
}
