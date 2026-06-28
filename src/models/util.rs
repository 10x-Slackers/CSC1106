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
