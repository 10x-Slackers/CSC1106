use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};

use crate::entity::account;
use crate::models::error::AppError;

#[derive(serde::Serialize)]
pub struct AccountOption {
    pub id: i32,
    pub name: String,
}

/// Find an account by exact name.
pub async fn find_by_name<C: ConnectionTrait>(
    db: &C,
    name: &str,
) -> Result<Option<account::Model>, AppError> {
    let acct = account::Entity::find()
        .filter(account::Column::Name.eq(name))
        .one(db)
        .await?;
    Ok(acct)
}

/// Find the "Accounts Receivable" and "Sales Revenue" accounts for posting journal entries.
pub async fn find_ar_and_sr<C: ConnectionTrait>(
    db: &C,
) -> Result<(account::Model, account::Model), AppError> {
    let ar = find_by_name(db, "Accounts Receivable")
        .await?
        .ok_or(AppError::NotFound)?;
    let sales = find_by_name(db, "Sales Revenue")
        .await?
        .ok_or(AppError::NotFound)?;
    Ok((ar, sales))
}

/// Find the "Operating Expenses" and "Accounts Payable" accounts for posting claim journal entries.
pub async fn find_oe_and_ap<C: ConnectionTrait>(
    db: &C,
) -> Result<(account::Model, account::Model), AppError> {
    let oe = find_by_name(db, "Operating Expenses")
        .await?
        .ok_or(AppError::NotFound)?;
    let ap = find_by_name(db, "Accounts Payable")
        .await?
        .ok_or(AppError::NotFound)?;
    Ok((oe, ap))
}

/// List all accounts ordered by name, for use in payment form dropdowns.
pub async fn list_accounts(db: &DatabaseConnection) -> Result<Vec<AccountOption>, AppError> {
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
