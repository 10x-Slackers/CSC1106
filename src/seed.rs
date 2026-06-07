use sea_orm::DatabaseConnection;

use crate::entity::role::Role;
use crate::models::user::User;

pub async fn seed(db: &DatabaseConnection) {
    // Check if already seeded
    if User::find_by_email(db, "admin@example.com")
        .await
        .expect("DB query failed")
        .is_some()
    {
        return;
    }

    // Create users
    // TODO: Don't hardcode credentials
    User::create(
        db,
        "admin@example.com",
        "Administrator",
        "P@ssw0rd",
        Role::Admin,
    )
    .await
    .expect("Failed to seed admin user");

    User::create(
        db,
        "john@example.com",
        "John Doe",
        "P@ssw0rd",
        Role::Accountant,
    )
    .await
    .expect("Failed to seed accountant user");
}
