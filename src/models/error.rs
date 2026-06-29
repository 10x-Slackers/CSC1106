use argon2::password_hash::Error as HashError;
use rust_decimal::Decimal;
use sea_orm::DbErr;

#[derive(Debug)]
pub enum AppError {
    Database(DbErr),
    NotFound,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Database(e) => write!(f, "Database error: {}", e),
            AppError::NotFound => write!(f, "Not found"),
        }
    }
}

impl From<DbErr> for AppError {
    fn from(e: DbErr) -> Self {
        AppError::Database(e)
    }
}

impl std::error::Error for AppError {}

#[derive(Debug)]
pub enum AuthError {
    InvalidCredentials,
    NotFound,
    HashError(HashError),
    Database(DbErr),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidCredentials => write!(f, "Invalid credentials"),
            AuthError::NotFound => write!(f, "User not found"),
            AuthError::HashError(e) => write!(f, "Password hash error: {}", e),
            AuthError::Database(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl From<DbErr> for AuthError {
    fn from(e: DbErr) -> Self {
        AuthError::Database(e)
    }
}

impl From<HashError> for AuthError {
    fn from(e: HashError) -> Self {
        AuthError::HashError(e)
    }
}

impl std::error::Error for AuthError {}

#[derive(Debug)]
pub enum PostingError {
    UnbalancedEntry {
        total_debits: Decimal,
        total_credits: Decimal,
    },
    NoLines,
    NonPositiveAmount {
        amount: Decimal,
    },
    Database(DbErr),
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
            PostingError::NonPositiveAmount { amount } => {
                write!(f, "Amount must be positive, got: {}", amount)
            }
            PostingError::Database(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl From<DbErr> for PostingError {
    fn from(e: DbErr) -> Self {
        PostingError::Database(e)
    }
}

impl std::error::Error for PostingError {}

#[derive(Debug)]
pub enum PaymentCreateError {
    SameAccount,
    Database(DbErr),
    Posting(PostingError),
}

impl std::fmt::Display for PaymentCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentCreateError::SameAccount => {
                write!(f, "From and to accounts must differ")
            }
            PaymentCreateError::Database(e) => write!(f, "Database error: {e}"),
            PaymentCreateError::Posting(e) => write!(f, "Posting failed: {e}"),
        }
    }
}

impl From<DbErr> for PaymentCreateError {
    fn from(e: DbErr) -> Self {
        PaymentCreateError::Database(e)
    }
}

impl From<PostingError> for PaymentCreateError {
    fn from(e: PostingError) -> Self {
        PaymentCreateError::Posting(e)
    }
}

impl From<InvoiceError> for PaymentCreateError {
    fn from(e: InvoiceError) -> Self {
        match e {
            InvoiceError::Posting(e) => PaymentCreateError::Posting(e),
            InvoiceError::Database(e) => PaymentCreateError::Database(e),
            other => PaymentCreateError::Database(sea_orm::DbErr::Custom(format!("{other}"))),
        }
    }
}

impl std::error::Error for PaymentCreateError {}

#[derive(Debug)]
pub enum InvoiceError {
    NotEditable,
    InvalidStatusTransition { from: String, to: String },
    NoLineItems,
    VoidWithPayments,
    AlreadyVoided,
    ValidationError(String),
    Posting(PostingError),
    Database(DbErr),
}

impl std::fmt::Display for InvoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvoiceError::NotEditable => write!(f, "Invoice is not editable in its current status"),
            InvoiceError::InvalidStatusTransition { from, to } => {
                write!(f, "Cannot transition from {from} to {to}")
            }
            InvoiceError::NoLineItems => write!(f, "Invoice must have at least one line item"),
            InvoiceError::VoidWithPayments => {
                write!(f, "Cannot void invoice with existing payments")
            }
            InvoiceError::AlreadyVoided => write!(f, "Invoice is already voided"),
            InvoiceError::ValidationError(msg) => write!(f, "{msg}"),
            InvoiceError::Posting(e) => write!(f, "Posting failed: {e}"),
            InvoiceError::Database(e) => write!(f, "Database error: {e}"),
        }
    }
}

impl From<DbErr> for InvoiceError {
    fn from(e: DbErr) -> Self {
        InvoiceError::Database(e)
    }
}

impl From<PostingError> for InvoiceError {
    fn from(e: PostingError) -> Self {
        InvoiceError::Posting(e)
    }
}

impl From<AppError> for InvoiceError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::Database(db_err) => InvoiceError::Database(db_err),
            AppError::NotFound => {
                InvoiceError::Database(sea_orm::DbErr::RecordNotFound("not found".into()))
            }
        }
    }
}

impl std::error::Error for InvoiceError {}
