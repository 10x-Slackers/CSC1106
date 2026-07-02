use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, JoinType, QueryFilter,
    QuerySelect, RelationTrait, Set,
};

use crate::entity::journal_entry as journal_entry_entity;
use crate::entity::journal_entry::SourceDocument;
use crate::entity::journal_entry_line as journal_entry_line_entity;
use crate::entity::journal_entry_line::EntrySide;
use crate::models::error::{AppError, PostingError};

#[derive(Clone)]
/// Input for a single debit or credit line within a journal entry.
pub struct JournalEntryLineInput {
    pub account_id: i32,
    pub entry_side: EntrySide,
    pub amount: Decimal,
    pub description: Option<String>,
}

/// Centralised posting service for the double-entry accounting engine.
pub struct PostingService;

impl PostingService {
    /// Create and post a journal entry within an existing transaction.
    pub async fn post_entry_in<C: ConnectionTrait>(
        db: &C,
        lines: Vec<JournalEntryLineInput>,
        source: SourceDocument,
        created_by_user_id: i32,
    ) -> Result<journal_entry_entity::Model, PostingError> {
        Self::validate_lines(&lines)?;

        let now = chrono::Utc::now().naive_utc();

        let (payment_id, claim_id, invoice_id) = source.to_fks();

        let journal_entry = journal_entry_entity::ActiveModel {
            payment_id: Set(payment_id),
            claim_id: Set(claim_id),
            invoice_id: Set(invoice_id),
            created_by_user_id: Set(created_by_user_id),
            created_at: Set(now),
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

    /// Validate amounts are positive, and debit and credit are balanced.
    fn validate_lines(lines: &[JournalEntryLineInput]) -> Result<(), PostingError> {
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

        let total_debits: Decimal = lines
            .iter()
            .filter(|l| l.entry_side == EntrySide::Debit)
            .map(|l| l.amount)
            .sum();
        let total_credits: Decimal = lines
            .iter()
            .filter(|l| l.entry_side == EntrySide::Credit)
            .map(|l| l.amount)
            .sum();

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
    from: Option<chrono::NaiveDateTime>,
    up_to: Option<chrono::NaiveDateTime>,
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
