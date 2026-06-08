use sea_orm::sea_query::OnConflict;
use sea_orm::{DatabaseConnection, EntityTrait, Set};

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
    use crate::entity::account as account_entity;

    let now = chrono::Utc::now().naive_utc();

    // Default chart of accounts
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

    let active_models: Vec<account_entity::ActiveModel> = accounts
        .into_iter()
        .map(
            |(name, category, normal_balance)| account_entity::ActiveModel {
                name: Set(name.to_string()),
                category: Set(category),
                normal_balance: Set(normal_balance),
                created_at: Set(now),
                ..Default::default()
            },
        )
        .collect();

    // Insert all accounts, skip any that already exist
    let result = account_entity::Entity::insert_many(active_models)
        .on_conflict(
            OnConflict::column(account_entity::Column::Name)
                .do_nothing()
                .to_owned(),
        )
        .exec(db)
        .await;

    match result {
        Ok(_) => {}
        Err(sea_orm::DbErr::RecordNotInserted) => {}
        Err(e) => panic!("Failed to seed chart of accounts: {e}"),
    }
}
