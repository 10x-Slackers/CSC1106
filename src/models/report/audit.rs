use chrono::NaiveDateTime;
use sea_orm::DatabaseConnection;

use crate::entity::journal_entry::Model as JournalEntry;
use crate::models::error::AppError;
pub use crate::models::journal_entry::AuditEntry;

#[derive(serde::Serialize)]
pub struct AuditReport {
    pub year: i32,
    pub from: NaiveDateTime,
    pub to: NaiveDateTime,
    pub entries: Vec<AuditEntry>,
    pub total_entries: usize,
}

pub struct AuditStatement;

impl AuditStatement {
    pub async fn compute(db: &DatabaseConnection, year: i32) -> Result<AuditReport, AppError> {
        let from = chrono::NaiveDate::from_ymd_opt(year, 1, 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .ok_or_else(|| AppError::InvalidArgument(format!("Invalid year: {year}")))?;
        let to = chrono::NaiveDate::from_ymd_opt(year, 12, 31)
            .and_then(|d| d.and_hms_opt(23, 59, 59))
            .ok_or_else(|| AppError::InvalidArgument(format!("Invalid year: {year}")))?;
        let entries = JournalEntry::list(db, from, to).await?;
        let total_entries = entries.len();
        Ok(AuditReport {
            year,
            from,
            to,
            entries,
            total_entries,
        })
    }
}
