use std::collections::HashMap;

use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, JoinType, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, RelationTrait,
};

use crate::entity::account;
use crate::entity::account::AccountCategory;
use crate::entity::journal_entry;
use crate::entity::journal_entry_line;
use crate::entity::journal_entry_line::EntrySide;
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

#[derive(Clone)]
pub struct AccountBalance {
    pub account: account::Model,
    pub balance: Decimal,
}

/// Compute signed balances for accounts in the given categories.
/// For Debit-normal accounts = debits - credits; for Credit-normal = credits - debits.
/// Returns all accounts in the categories sorted by name.
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
    let lines = journal_entry_line::Entity::find()
        .join(
            JoinType::InnerJoin,
            journal_entry_line::Relation::JournalEntry.def(),
        )
        .filter(journal_entry_line::Column::AccountId.is_in(account_ids))
        .apply_if(from, |q, dt| {
            q.filter(journal_entry::Column::CreatedAt.gte(dt))
        })
        .apply_if(up_to, |q, dt| {
            q.filter(journal_entry::Column::CreatedAt.lte(dt))
        })
        .all(db)
        .await?;

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
