use std::collections::HashMap;

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, Order, QueryFilter, QueryOrder};

use crate::entity::account;
use crate::entity::claim;
use crate::entity::invoice;
use crate::entity::journal_entry as journal_entry_entity;
use crate::entity::journal_entry_line as journal_entry_line_entity;
use crate::entity::journal_entry_line::EntrySide;
use crate::entity::user;
use crate::models::error::AppError;

/// A single journal entry line enriched with the account name for display.
#[derive(serde::Serialize, Clone)]
pub struct AuditLine {
    pub account_name: String,
    pub entry_side: EntrySide,
    pub amount: Decimal,
    pub description: Option<String>,
}

/// A journal entry enriched with its lines, poster name, and source document reference.
#[derive(serde::Serialize, Clone)]
pub struct AuditEntry {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub posted_by_name: Option<String>,
    pub source: String,
    pub source_link: Option<String>,
    pub lines: Vec<AuditLine>,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
}

/// List all posted journal entries within a range, ordered chronologically then by id.
/// Each entry enriched with its lines, account names, poster name, and source document reference.
pub async fn list_for_audit<C: ConnectionTrait>(
    db: &C,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Result<Vec<AuditEntry>, AppError> {
    let entries = fetch_entries(db, from, to).await?;
    if entries.is_empty() {
        return Ok(Vec::new());
    }

    let entry_ids: Vec<i32> = entries.iter().map(|e| e.id).collect();

    let lines = fetch_lines(db, entry_ids).await?;
    let account_names = build_account_name_map(db, &lines).await?;
    let user_names = build_user_name_map(db, &entries).await?;
    let (invoice_map, claim_map) = build_source_maps(db, &entries).await?;

    let lines_by_entry = group_lines(lines, &account_names);

    let result = entries
        .into_iter()
        .map(|entry| {
            let lines = lines_by_entry.get(&entry.id).cloned().unwrap_or_default();
            build_audit_entry(entry, lines, &user_names, &invoice_map, &claim_map)
        })
        .collect();

    Ok(result)
}

async fn fetch_entries<C: ConnectionTrait>(
    db: &C,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Result<Vec<journal_entry_entity::Model>, AppError> {
    journal_entry_entity::Entity::find()
        .filter(journal_entry_entity::Column::CreatedAt.gte(from))
        .filter(journal_entry_entity::Column::CreatedAt.lte(to))
        .order_by(journal_entry_entity::Column::CreatedAt, Order::Asc)
        .order_by(journal_entry_entity::Column::Id, Order::Asc)
        .all(db)
        .await
        .map_err(AppError::from)
}

async fn fetch_lines<C: ConnectionTrait>(
    db: &C,
    entry_ids: Vec<i32>,
) -> Result<Vec<journal_entry_line_entity::Model>, AppError> {
    journal_entry_line_entity::Entity::find()
        .filter(journal_entry_line_entity::Column::EntryId.is_in(entry_ids))
        .all(db)
        .await
        .map_err(AppError::from)
}

async fn build_account_name_map<C: ConnectionTrait>(
    db: &C,
    lines: &[journal_entry_line_entity::Model],
) -> Result<HashMap<i32, String>, AppError> {
    let account_ids: Vec<i32> = unique_ids(lines.iter().map(|l| l.account_id));
    if account_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let accounts = account::Entity::find()
        .filter(account::Column::Id.is_in(account_ids))
        .all(db)
        .await
        .map_err(AppError::from)?;
    Ok(accounts.into_iter().map(|a| (a.id, a.name)).collect())
}

async fn build_user_name_map<C: ConnectionTrait>(
    db: &C,
    entries: &[journal_entry_entity::Model],
) -> Result<HashMap<i32, String>, AppError> {
    let user_ids: Vec<i32> = unique_ids(entries.iter().map(|e| e.created_by_user_id));
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let users = user::Entity::find()
        .filter(user::Column::Id.is_in(user_ids))
        .all(db)
        .await
        .map_err(AppError::from)?;
    Ok(users.into_iter().map(|u| (u.id, u.name)).collect())
}

/// Batch-load invoice and claim labels for source-document display.
async fn build_source_maps<C: ConnectionTrait>(
    db: &C,
    entries: &[journal_entry_entity::Model],
) -> Result<(HashMap<i32, String>, HashMap<i32, String>), AppError> {
    let invoice_ids: Vec<i32> = unique_ids(entries.iter().filter_map(|e| e.invoice_id));
    let claim_ids: Vec<i32> = unique_ids(entries.iter().filter_map(|e| e.claim_id));

    let invoice_map = if invoice_ids.is_empty() {
        HashMap::new()
    } else {
        invoice::Entity::find()
            .filter(invoice::Column::Id.is_in(invoice_ids))
            .all(db)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(|inv| (inv.id, inv.invoice_no))
            .collect()
    };

    let claim_map = if claim_ids.is_empty() {
        HashMap::new()
    } else {
        claim::Entity::find()
            .filter(claim::Column::Id.is_in(claim_ids))
            .all(db)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(|c| (c.id, c.title))
            .collect()
    };

    Ok((invoice_map, claim_map))
}

/// Group raw lines by entry ID, resolving account names.
fn group_lines(
    lines: Vec<journal_entry_line_entity::Model>,
    account_names: &HashMap<i32, String>,
) -> HashMap<i32, Vec<AuditLine>> {
    let mut map: HashMap<i32, Vec<AuditLine>> = HashMap::new();
    for line in lines {
        let audit_line = AuditLine {
            account_name: account_names
                .get(&line.account_id)
                .cloned()
                .unwrap_or_else(|| format!("Account #{}", line.account_id)),
            entry_side: line.entry_side,
            amount: line.amount,
            description: line.description,
        };
        map.entry(line.entry_id).or_default().push(audit_line);
    }
    map
}

fn build_audit_entry(
    entry: journal_entry_entity::Model,
    lines: Vec<AuditLine>,
    user_names: &HashMap<i32, String>,
    invoice_map: &HashMap<i32, String>,
    claim_map: &HashMap<i32, String>,
) -> AuditEntry {
    let total_debit: Decimal = lines
        .iter()
        .filter(|l| l.entry_side == EntrySide::Debit)
        .map(|l| l.amount)
        .sum();
    let total_credit: Decimal = lines
        .iter()
        .filter(|l| l.entry_side == EntrySide::Credit)
        .map(|l| l.amount)
        .sum();

    let source = format_source(&entry, invoice_map, claim_map);
    let source_link = entry
        .invoice_id
        .map(|id| format!("/invoices/{id}"))
        .or_else(|| entry.payment_id.map(|id| format!("/payments/{id}")))
        .or_else(|| entry.claim_id.map(|id| format!("/claims/{id}")));

    AuditEntry {
        id: entry.id,
        created_at: entry.created_at,
        posted_by_name: user_names.get(&entry.created_by_user_id).cloned(),
        source,
        source_link,
        lines,
        total_debit,
        total_credit,
    }
}

/// Format a human-readable source label for a journal entry.
fn format_source(
    entry: &journal_entry_entity::Model,
    invoice_map: &HashMap<i32, String>,
    claim_map: &HashMap<i32, String>,
) -> String {
    if let Some(inv_id) = entry.invoice_id {
        match invoice_map.get(&inv_id) {
            Some(no) => format!("Invoice {}", no),
            None => format!("Invoice #{}", inv_id),
        }
    } else if let Some(pay_id) = entry.payment_id {
        format!("Payment #{}", pay_id)
    } else if let Some(cl_id) = entry.claim_id {
        match claim_map.get(&cl_id) {
            Some(title) => format!("Claim: {}", title),
            None => format!("Claim #{}", cl_id),
        }
    } else {
        "Manual entry".to_string()
    }
}

/// Collect, sort, and deduplicate IDs for `is_in` queries.
fn unique_ids(iter: impl Iterator<Item = i32>) -> Vec<i32> {
    let mut ids: Vec<i32> = iter.collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}
