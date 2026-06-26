use sea_orm::sea_query::OnConflict;
use sea_orm::{DatabaseConnection, EntityTrait, Set};

use crate::entity::account::{AccountCategory, NormalBalance};
use crate::entity::party::{PartyStatus, PartyType};
use crate::entity::user::Role;
use crate::models::user::User;

/// Insert default users if the database is empty.
pub async fn seed_users(db: &DatabaseConnection) {
    if User::find_by_email(db, "admin@example.com")
        .await
        .expect("DB query failed")
        .is_some()
    {
        return;
    }

    // TODO: Don't hardcode credentials
    let users = [
        (
            "admin@example.com",
            "Administrator",
            "P@ssw0rd",
            Role::Admin,
            false,
        ),
        (
            "john@example.com",
            "John Doe",
            "P@ssw0rd",
            Role::Accountant,
            false,
        ),
        (
            "staff@example.com",
            "Staff Member",
            "P@ssw0rd",
            Role::Staff,
            true,
        ),
    ];

    for (email, name, password, role, disabled) in users {
        let user = User::create(db, email, name, password, role)
            .await
            .expect("Failed to seed user");
        if disabled {
            user.set_disabled(db, &crate::middleware::auth::UserCache::new(), true)
                .await
                .expect("Failed to disable seeded user");
        }
    }
}

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

/// Insert sample parties if the database is empty.
pub async fn seed_parties(db: &DatabaseConnection) {
    use crate::models::party::Party;

    if Party::find_by_email(db, "alice@acme.com")
        .await
        .expect("DB query failed")
        .is_some()
    {
        return;
    }

    let parties = [
        (
            PartyType::Customer,
            "Alice Tan",
            Some("Acme Pte Ltd"),
            "alice@acme.com",
            "91234567",
            "123 Orchard Road",
            PartyStatus::Active,
        ),
        (
            PartyType::Vendor,
            "Bob Lim",
            Some("Lim Supplies"),
            "bob@supplies.com",
            "82345678",
            "456 Industrial Ave",
            PartyStatus::Active,
        ),
        (
            PartyType::Vendor,
            "David Chen",
            Some("Chen Trading"),
            "david@oldvendor.com",
            "84567890",
            "321 River Valley",
            PartyStatus::Inactive,
        ),
    ];

    for (party_type, name, company, email, phone, address, status) in parties {
        let party = Party::create(db, party_type, name, company, email, phone, address)
            .await
            .expect("Failed to seed party");
        if status == PartyStatus::Inactive {
            party
                .set_status(db, PartyStatus::Inactive)
                .await
                .expect("Failed to deactivate seeded party");
        }
    }
}
