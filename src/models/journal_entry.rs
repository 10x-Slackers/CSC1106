use std::collections::HashMap;

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, JoinType, Order, QueryFilter,
    QueryOrder, QuerySelect, RelationTrait, Set,
};

use crate::entity::journal_entry as journal_entry_entity;
use crate::entity::journal_entry::SourceDocument;
use crate::entity::journal_entry_line as journal_entry_line_entity;
use crate::entity::journal_entry_line::EntrySide;
use crate::models::error::{AppError, PostingError};

pub use journal_entry_entity::Model as JournalEntry;

/// Input for a single debit or credit line within a journal entry.
#[derive(Clone)]
pub struct JournalEntryLineInput {
    pub account_id: i32,
    pub entry_side: EntrySide,
    pub amount: Decimal,
    pub description: Option<String>,
}

/// A single journal entry line paired with its account name, for display.
#[derive(serde::Serialize, Clone)]
pub struct AuditLine {
    pub account_name: String,
    pub entry_side: EntrySide,
    pub amount: Decimal,
    pub description: Option<String>,
}

/// A journal entry with its debit/credit lines, the name of who created it, and its source document, for display.
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

impl JournalEntry {
    /// Create a journal entry and insert its debit/credit lines, within an existing transaction.
    pub async fn create<C: ConnectionTrait>(
        db: &C,
        lines: Vec<JournalEntryLineInput>,
        source: SourceDocument,
        created_by_user_id: i32,
    ) -> Result<JournalEntry, PostingError> {
        JournalEntryLineInput::validate_all(&lines)?;

        let (payment_id, claim_id, invoice_id) = source.to_fks();

        let journal_entry = journal_entry_entity::ActiveModel {
            payment_id: Set(payment_id),
            claim_id: Set(claim_id),
            invoice_id: Set(invoice_id),
            created_by_user_id: Set(created_by_user_id),
            ..Default::default()
        }
        .insert(db)
        .await?;

        let line_models: Vec<journal_entry_line_entity::ActiveModel> = lines
            .into_iter()
            .map(|line| journal_entry_line_entity::ActiveModel {
                entry_id: Set(journal_entry.id),
                account_id: Set(line.account_id),
                entry_side: Set(line.entry_side),
                amount: Set(line.amount),
                description: Set(line.description),
                ..Default::default()
            })
            .collect();

        journal_entry_line_entity::Entity::insert_many(line_models)
            .exec(db)
            .await?;

        Ok(journal_entry)
    }

    /// List all journal entries created within a range, ordered chronologically then by id.
    /// Each entry comes with its debit/credit lines, account names, creator's name, and source document, for display.
    pub async fn list<C: ConnectionTrait>(
        db: &C,
        from: NaiveDateTime,
        to: NaiveDateTime,
    ) -> Result<Vec<AuditEntry>, AppError> {
        let entries = fetch_entries(db, from, to).await?;
        if entries.is_empty() {
            return Ok(Vec::new());
        }

        let entry_ids: Vec<i32> = entries.iter().map(|e| e.id).collect();

        let (lines, user_names, (invoice_map, claim_map)) = {
            let lines_fut = fetch_lines(db, entry_ids);
            let user_fut = build_user_name_map(db, &entries);
            let source_fut = build_source_maps(db, &entries);
            futures::try_join!(lines_fut, user_fut, source_fut)?
        };
        let account_names = build_account_name_map(db, &lines).await?;

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
}

impl JournalEntryLineInput {
    /// Validate journal entry lines: non-empty, positive amounts, balanced debits/credits.
    pub fn validate_all(lines: &[Self]) -> Result<(), PostingError> {
        if lines.is_empty() {
            return Err(PostingError::NoLines);
        }

        for line in lines {
            if line.amount <= Decimal::ZERO {
                return Err(PostingError::NonPositiveAmount {
                    amount: line.amount,
                });
            }
        }

        let (total_debits, total_credits) = totals(lines.iter().map(|l| (&l.entry_side, l.amount)));

        if total_debits != total_credits {
            return Err(PostingError::UnbalancedEntry {
                total_debits,
                total_credits,
            });
        }

        Ok(())
    }
}

/// Fetch journal entry lines for the given account IDs, joined to their parent entry
/// and filtered by optional date range on the entry's created_at timestamp.
pub async fn lines_for_accounts<C: ConnectionTrait>(
    db: &C,
    account_ids: Vec<i32>,
    from: Option<NaiveDateTime>,
    up_to: Option<NaiveDateTime>,
) -> Result<Vec<journal_entry_line_entity::Model>, AppError> {
    let mut query = journal_entry_line_entity::Entity::find()
        .join(
            JoinType::InnerJoin,
            journal_entry_line_entity::Relation::JournalEntry.def(),
        )
        .filter(journal_entry_line_entity::Column::AccountId.is_in(account_ids));

    if let Some(dt) = from {
        query = query.filter(journal_entry_entity::Column::CreatedAt.gte(dt));
    }
    if let Some(dt) = up_to {
        query = query.filter(journal_entry_entity::Column::CreatedAt.lte(dt));
    }

    query.all(db).await.map_err(AppError::from)
}

/// Sum debits and credits from an iterator of (side, amount) pairs.
fn totals<'a>(lines: impl Iterator<Item = (&'a EntrySide, Decimal)>) -> (Decimal, Decimal) {
    let mut total_debit = Decimal::ZERO;
    let mut total_credit = Decimal::ZERO;
    for (side, amount) in lines {
        match side {
            EntrySide::Debit => total_debit += amount,
            EntrySide::Credit => total_credit += amount,
        }
    }
    (total_debit, total_credit)
}

/// Fetch journal entries created within the given date range.
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

/// Fetch all lines belonging to the given journal entries.
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

/// Build a map of account id to name, for the accounts used in the given lines.
async fn build_account_name_map<C: ConnectionTrait>(
    db: &C,
    lines: &[journal_entry_line_entity::Model],
) -> Result<HashMap<i32, String>, AppError> {
    let account_ids: Vec<i32> = unique_ids(lines.iter().map(|l| l.account_id));
    crate::models::account::name_map_by_ids(db, account_ids).await
}

/// Build a map of user id to name, for the users who created the given entries.
async fn build_user_name_map<C: ConnectionTrait>(
    db: &C,
    entries: &[journal_entry_entity::Model],
) -> Result<HashMap<i32, String>, AppError> {
    let user_ids: Vec<i32> = unique_ids(entries.iter().map(|e| e.created_by_user_id));
    crate::models::user::name_map_by_ids(db, user_ids).await
}

/// Batch-load invoice and claim labels for source-document display.
async fn build_source_maps<C: ConnectionTrait>(
    db: &C,
    entries: &[journal_entry_entity::Model],
) -> Result<(HashMap<i32, String>, HashMap<i32, String>), AppError> {
    let invoice_ids: Vec<i32> = unique_ids(entries.iter().filter_map(|e| e.invoice_id));
    let claim_ids: Vec<i32> = unique_ids(entries.iter().filter_map(|e| e.claim_id));

    let invoice_map = crate::models::invoice::invoice_no_map_by_ids(db, invoice_ids).await?;
    let claim_map = crate::models::claim::title_map_by_ids(db, claim_ids)
        .await
        .map_err(AppError::from)?;

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

/// Build an `AuditEntry` from a journal entry, its lines, and resolved names/links, for display.
fn build_audit_entry(
    entry: journal_entry_entity::Model,
    lines: Vec<AuditLine>,
    user_names: &HashMap<i32, String>,
    invoice_map: &HashMap<i32, String>,
    claim_map: &HashMap<i32, String>,
) -> AuditEntry {
    let (total_debit, total_credit) = totals(lines.iter().map(|l| (&l.entry_side, l.amount)));

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
