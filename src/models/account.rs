use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder};

use crate::entity::account;
use crate::models::user::AuthError;

#[derive(serde::Serialize)]
pub struct AccountOption {
    pub id: i32,
    pub name: String,
}

/// List all accounts ordered by name, for use in payment form dropdowns.
pub async fn list_accounts(db: &DatabaseConnection) -> Result<Vec<AccountOption>, AuthError> {
    let accounts = account::Entity::find()
        .order_by_asc(account::Column::Name)
        .all(db)
        .await?;
    Ok(accounts
        .into_iter()
        .map(|a| AccountOption {
            id: a.id,
            name: a.name,
        })
        .collect())
}
