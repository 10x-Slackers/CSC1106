use std::io::{self, Write};

use sea_orm::sea_query::OnConflict;
use sea_orm::{DatabaseConnection, EntityTrait, Set};

use crate::entity::account::{AccountCategory, NormalBalance};
use crate::entity::user::Role;
use crate::models::user::User;

/// Insert a default chart of accounts, skipping duplicates.
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

/// Insert default claim categories, skipping duplicates.
pub async fn seed_claim_categories(db: &DatabaseConnection) {
    use crate::entity::claim_category as claim_category_entity;

    let categories = ["Travel", "Meals", "Supplies", "Misc"];

    let active_models: Vec<claim_category_entity::ActiveModel> = categories
        .into_iter()
        .map(|name| claim_category_entity::ActiveModel {
            name: Set(name.to_string()),
            ..Default::default()
        })
        .collect();

    let result = claim_category_entity::Entity::insert_many(active_models)
        .on_conflict(
            OnConflict::column(claim_category_entity::Column::Name)
                .do_nothing()
                .to_owned(),
        )
        .exec(db)
        .await;

    match result {
        Ok(_) => {}
        Err(sea_orm::DbErr::RecordNotInserted) => {}
        Err(e) => panic!("Failed to seed claim categories: {e}"),
    }
}

/// Prompt the operator interactively to create an admin user.
pub async fn create_admin_interactively(db: &DatabaseConnection) -> User {
    println!("\nNo users found, proceeding with admin account creation.\n");

    loop {
        let email = prompt("Admin email: ");
        if !email.contains('@') {
            println!("Email must contain '@'.");
            continue;
        }

        let name = prompt("Admin name: ");
        if name.is_empty() {
            println!("Name cannot be empty.");
            continue;
        }

        let password = prompt("Password: ");
        if password.len() < 8 {
            println!("Password must be at least 8 characters.");
            continue;
        }

        let confirm = prompt("Confirm password: ");
        if password != confirm {
            println!("Passwords do not match.");
            continue;
        }

        match User::create(db, &email, &name, &password, Role::Admin).await {
            Ok(user) => {
                println!("Created admin: {} ({})", user.name, user.email);
                return user;
            }
            Err(e) => {
                println!("Failed to create admin: {e}");
                continue;
            }
        }
    }
}

/// Print a prompt, flush stdout, and read a line from stdin.
fn prompt(label: &str) -> String {
    print!("{label}");
    io::stdout().flush().expect("Failed to flush stdout");
    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .expect("Failed to read stdin");
    buf.trim().to_string()
}
