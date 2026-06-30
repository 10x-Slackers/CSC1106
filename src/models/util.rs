use sea_orm::DbErr;

/// Escape `\`, `%` and `_` and wrap in `%...%` for a substring LIKE match.
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

/// True if a `DbErr` is a SQLite unique constraint violation.
pub fn is_unique_violation(err: &DbErr) -> bool {
    err.to_string().contains("UNIQUE constraint failed")
}

/// Number of rows shown per page in list views.
pub const PER_PAGE: u64 = 10;

/// Clamp `page` to a valid 1-indexed range given SeaORM's `num_pages` (u64).
/// Returns `(total_pages, current_page)`.
pub fn clamp_pagination(page: u32, num_pages: u64) -> (u32, u32) {
    let total_pages = num_pages.min(u32::MAX as u64) as u32;
    let total_pages = total_pages.max(1);
    (total_pages, page.min(total_pages))
}
