use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

use crate::entity::account::{AccountCategory, NormalBalance};
use crate::entity::user::Role;
use crate::models::user::User;

pub async fn seed_users(db: &DatabaseConnection) {
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

pub async fn seed_accounts(db: &DatabaseConnection) {
    // Check if accounts already exist
    use crate::entity::account as account_entity;

    if account_entity::Entity::find()
        .one(db)
        .await
        .expect("DB query failed")
        .is_some()
    {
        return;
    }

    let now = chrono::Utc::now().naive_utc();

    let accounts = [
        ("Cash", AccountCategory::Asset, NormalBalance::Debit),
        (
            "Accounts Receivable",
            AccountCategory::Asset,
            NormalBalance::Debit,
        ),
        (
            "Cost of Goods Sold",
            AccountCategory::Expense,
            NormalBalance::Debit,
        ),
        (
            "Operating Expenses",
            AccountCategory::Expense,
            NormalBalance::Debit,
        ),
        (
            "Accounts Payable",
            AccountCategory::Liability,
            NormalBalance::Credit,
        ),
        (
            "Owner's Equity",
            AccountCategory::Equity,
            NormalBalance::Credit,
        ),
        (
            "Retained Earnings",
            AccountCategory::Equity,
            NormalBalance::Credit,
        ),
        (
            "Sales Revenue",
            AccountCategory::Revenue,
            NormalBalance::Credit,
        ),
    ];

    for (name, category, normal_balance) in accounts {
        let account = account_entity::ActiveModel {
            name: Set(name.to_string()),
            category: Set(category),
            normal_balance: Set(normal_balance),
            created_at: Set(now),
            ..Default::default()
        };

        account
            .insert(db)
            .await
            .unwrap_or_else(|e| panic!("Failed to seed account '{}': {}", name, e));
    }
}
