use sea_orm::DatabaseConnection;

use crate::entity::role::Role;
use crate::models::user::{create_user, find_user};

pub async fn seed(db: &DatabaseConnection) {
    // Check if already seeded
    if find_user(db, "admin@example.com")
        .await
        .expect("DB query failed")
        .is_some()
    {
        return;
    }

    // Create users
    // TODO: Don't hardcode credentials
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
