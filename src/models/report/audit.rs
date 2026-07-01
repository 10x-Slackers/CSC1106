use chrono::NaiveDateTime;
use sea_orm::DatabaseConnection;

use crate::models::error::AppError;
pub use crate::models::journal_entry::AuditEntry;
use crate::models::journal_entry::list_for_audit;

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
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let to = chrono::NaiveDate::from_ymd_opt(year, 12, 31)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();
        let entries = list_for_audit(db, from, to).await?;
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
