use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, Set, TransactionTrait};

use crate::entity::journal_entry as journal_entry_entity;
use crate::entity::journal_entry::SourceDocument;
use crate::entity::journal_entry_line as journal_entry_line_entity;
use crate::entity::journal_entry_line::EntrySide;

#[allow(dead_code)] // TODO: Remove when implementing journal entry UI
#[derive(Clone)]
pub struct JournalEntryLineInput {
    pub account_id: i32,
    pub entry_side: EntrySide,
    pub amount: Decimal,
    pub description: Option<String>,
}

#[allow(dead_code)] // TODO: Remove when implementing journal entry UI
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

#[allow(dead_code)] // TODO: Remove when implementing journal entry UI
/// Centralised posting service for the double-entry accounting engine.
pub struct PostingService;

#[allow(dead_code)] // TODO: Remove when implementing journal entry UI
impl PostingService {
    /// Create and post a journal entry with an atomic transaction.
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
                    let (payment_id, claim_id, invoice_id) = source.to_fks();

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

    /// Validate amounts are positive, and debit and credit are balanced.
    fn validate_lines(lines: &[JournalEntryLineInput]) -> Result<(), PostingError> {
        if lines.is_empty() {
            return Err(PostingError::NoLines);
        }

        for line in lines {
            if line.amount <= Decimal::ZERO {
                return Err(PostingError::DatabaseError(DbErr::Custom(format!(
                    "Amount must be positive, got: {}",
                    line.amount
                ))));
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
