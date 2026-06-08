use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, Set, TransactionTrait};

use crate::entity::journal_entry as journal_entry_entity;
use crate::entity::journal_entry_line as journal_entry_line_entity;
use crate::entity::journal_entry_line::EntrySide;

#[derive(Clone)]
pub struct JournalEntryLineInput {
    pub account_id: i32,
    pub entry_side: EntrySide,
    pub amount: Decimal,
    pub description: Option<String>,
}

/// Each journal entry traces back to at most one source document.
#[derive(Clone)]
pub enum SourceDocument {
    Payment { payment_id: i32 },
    Claim { claim_id: i32 },
    Invoice { invoice_id: i32 },
    Manual,
}

#[derive(Debug)]
pub enum PostingError {
    UnbalancedEntry {
        total_debits: Decimal,
        total_credits: Decimal,
    },
    NoLines,
    DatabaseError(DbErr),
}

impl std::fmt::Display for PostingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostingError::UnbalancedEntry {
                total_debits,
                total_credits,
            } => write!(
                f,
                "Unbalanced entry: total debits ({}) != total credits ({})",
                total_debits, total_credits
            ),
            PostingError::NoLines => write!(f, "Journal entry must have at least one line"),
            PostingError::DatabaseError(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl From<DbErr> for PostingError {
    fn from(e: DbErr) -> Self {
        PostingError::DatabaseError(e)
    }
}

/// Centralised posting service for the double-entry accounting engine.
pub struct PostingService;

impl PostingService {
    /// Create and post a journal entry in a single atomic transaction.
    pub async fn post_entry(
        db: &DatabaseConnection,
        lines: Vec<JournalEntryLineInput>,
        source: SourceDocument,
        created_by_user_id: i32,
    ) -> Result<journal_entry_entity::Model, PostingError> {
        Self::validate_lines(&lines)?;

        // Execute within a transaction
        let result = db
            .transaction::<_, journal_entry_entity::Model, PostingError>(|txn| {
                let lines = lines.clone();
                let source = source.clone();
                Box::pin(async move {
                    let now = chrono::Utc::now().naive_utc();

                    // Insert journal entry header
                    let (payment_id, claim_id, invoice_id) = Self::resolve_source_fks(&source);

                    let journal_entry = journal_entry_entity::ActiveModel {
                        payment_id: Set(payment_id),
                        claim_id: Set(claim_id),
                        invoice_id: Set(invoice_id),
                        created_by_user_id: Set(created_by_user_id),
                        created_at: Set(now),
                        ..Default::default()
                    }
                    .insert(txn)
                    .await?;

                    // Insert journal entry lines
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
                        .exec(txn)
                        .await?;

                    Ok(journal_entry)
                })
            })
            .await;

        result.map_err(|e| match e {
            sea_orm::TransactionError::Connection(e) => PostingError::DatabaseError(e),
            sea_orm::TransactionError::Transaction(e) => e,
        })
    }

    fn validate_lines(lines: &[JournalEntryLineInput]) -> Result<(), PostingError> {
        if lines.is_empty() {
            return Err(PostingError::NoLines);
        }

        // Check if positive amounts
        for line in lines {
            if line.amount <= Decimal::ZERO {
                return Err(PostingError::DatabaseError(DbErr::Custom(format!(
                    "Amount must be positive, got: {}",
                    line.amount
                ))));
            }
        }

        // Check debit and credit are balanced
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

    /// Map a SourceDocument variant to its nullable FK columns.
    fn resolve_source_fks(source: &SourceDocument) -> (Option<i32>, Option<i32>, Option<i32>) {
        match source {
            SourceDocument::Payment { payment_id } => (Some(*payment_id), None, None),
            SourceDocument::Claim { claim_id } => (None, Some(*claim_id), None),
            SourceDocument::Invoice { invoice_id } => (None, None, Some(*invoice_id)),
            SourceDocument::Manual => (None, None, None),
        }
    }
}
