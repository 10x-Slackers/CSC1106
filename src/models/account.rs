//! Look up accounts by name and calculate their balances.
//!
//! Authors: commit2main

use std::collections::HashMap;

use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

use crate::entity::account;
use crate::entity::account::AccountCategory;
use crate::entity::journal_entry_line::EntrySide;
use crate::models::error::AppError;
use crate::models::journal_entry;

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
    let (ar, sales) = futures::try_join!(
        find_by_name(db, "Accounts Receivable"),
        find_by_name(db, "Sales Revenue"),
    )?;
    Ok((
        ar.ok_or(AppError::NotFound)?,
        sales.ok_or(AppError::NotFound)?,
    ))
}

/// Find the "Operating Expenses" and "Accounts Payable" accounts for posting claim journal entries.
pub async fn find_oe_and_ap<C: ConnectionTrait>(
    db: &C,
) -> Result<(account::Model, account::Model), AppError> {
    let (oe, ap) = futures::try_join!(
        find_by_name(db, "Operating Expenses"),
        find_by_name(db, "Accounts Payable"),
    )?;
    Ok((oe.ok_or(AppError::NotFound)?, ap.ok_or(AppError::NotFound)?))
}

/// Find the "Cash" and "Accounts Receivable" accounts for posting invoice payment entries.
pub async fn find_cash_and_ar<C: ConnectionTrait>(
    db: &C,
) -> Result<(account::Model, account::Model), AppError> {
    let (cash, ar) = futures::try_join!(
        find_by_name(db, "Cash"),
        find_by_name(db, "Accounts Receivable"),
    )?;
    Ok((
        cash.ok_or(AppError::NotFound)?,
        ar.ok_or(AppError::NotFound)?,
    ))
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

#[derive(Clone)]
pub struct AccountBalance {
    pub account: account::Model,
    pub balance: Decimal,
}

/// Compute each account's balance.
pub async fn balances_by_category<C: ConnectionTrait>(
    db: &C,
    categories: &[AccountCategory],
    from: Option<chrono::NaiveDateTime>,
    up_to: Option<chrono::NaiveDateTime>,
) -> Result<Vec<AccountBalance>, AppError> {
    let accounts = account::Entity::find()
        .filter(account::Column::Category.is_in(categories.iter().cloned()))
        .order_by_asc(account::Column::Name)
        .all(db)
        .await?;

    if accounts.is_empty() {
        return Ok(Vec::new());
    }

    let account_ids: Vec<i32> = accounts.iter().map(|a| a.id).collect();

    // Fetch journal entry lines joined to their parent entry, filtered by date.
    let lines = journal_entry::lines_for_accounts(db, account_ids, from, up_to).await?;

    // Sum debits/credits per account.
    let mut totals: HashMap<i32, Decimal> = HashMap::new();
    for line in lines {
        let signed = match line.entry_side {
            EntrySide::Debit => line.amount,
            EntrySide::Credit => -line.amount,
        };
        *totals.entry(line.account_id).or_default() += signed;
    }

    // Build result, applying sign flip for Credit-normal accounts.
    let result = accounts
        .into_iter()
        .map(|acct| {
            let raw = totals.get(&acct.id).copied().unwrap_or(Decimal::ZERO);
            let balance = match acct.normal_balance {
                account::NormalBalance::Debit => raw,
                account::NormalBalance::Credit => -raw,
            };
            AccountBalance {
                account: acct,
                balance,
            }
        })
        .collect();

    Ok(result)
}

/// Build a map of account ID → name for the given IDs.
pub async fn name_map_by_ids<C: ConnectionTrait>(
    db: &C,
    ids: Vec<i32>,
) -> Result<HashMap<i32, String>, AppError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(i32, String)> = account::Entity::find()
        .select_only()
        .column(account::Column::Id)
        .column(account::Column::Name)
        .filter(account::Column::Id.is_in(ids))
        .into_tuple()
        .all(db)
        .await?;
    Ok(rows.into_iter().collect())
}
