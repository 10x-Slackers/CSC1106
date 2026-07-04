//! Utility functions for models.
//!
//! Authors: commit2main
use sea_orm::DbErr;

/// Escape string for SQL LIKE query.
pub fn like_pattern(q: &str) -> Option<String> {
    let trimmed = q.trim();
    if trimmed.is_empty() {
        return None;
    }
    let escaped = trimmed
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Some(format!("%{}%", escaped))
}

/// Trim and return `Some` if non-empty, else `None`.
pub fn non_empty(s: &str) -> Option<&str> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t) }
}

/// Check if a database error is a unique constraint violation.
pub fn is_unique_violation(err: &DbErr) -> bool {
    err.to_string().contains("UNIQUE constraint failed")
}

pub const PER_PAGE: u64 = 10;

/// Clamp the requested page number to the valid range of pages.
pub fn clamp_pagination(page: u32, num_pages: u64) -> (u32, u32) {
    let total_pages = num_pages.min(u32::MAX as u64) as u32;
    let total_pages = total_pages.max(1);
    (total_pages, page.min(total_pages))
}
