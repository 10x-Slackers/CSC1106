use sea_orm::DbErr;

/// True if a `DbErr` is a SQLite unique constraint violation.
pub fn is_unique_violation(err: &DbErr) -> bool {
    err.to_string().contains("UNIQUE constraint failed")
}
