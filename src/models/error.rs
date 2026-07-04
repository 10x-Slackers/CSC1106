//! Error types for the application
//!
//! Authors: commit2main
use argon2::password_hash::Error as HashError;
use rust_decimal::Decimal;
use sea_orm::DbErr;

use crate::models::util::is_unique_violation;

/// Application-level error types.
#[derive(Debug)]
pub enum AppError {
    Database(DbErr),
    NotFound,
    Duplicate,
    InvalidArgument(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Database(e) => write!(f, "Database error: {}", e),
            AppError::NotFound => write!(f, "Not found"),
            AppError::Duplicate => write!(f, "Duplicate record"),
            AppError::InvalidArgument(msg) => write!(f, "Invalid argument: {msg}"),
        }
    }
}

impl From<DbErr> for AppError {
    fn from(e: DbErr) -> Self {
        if is_unique_violation(&e) {
            AppError::Duplicate
        } else {
            AppError::Database(e)
        }
    }
}

impl std::error::Error for AppError {}

impl From<ClaimError> for AppError {
    fn from(e: ClaimError) -> Self {
        match e {
            ClaimError::Database(db_err) => AppError::Database(db_err),
            ClaimError::NotFound => AppError::NotFound,
            ClaimError::Duplicate => AppError::Duplicate,
            ClaimError::InvalidInput(msg) => AppError::InvalidArgument(msg),
            other => AppError::InvalidArgument(other.to_string()),
        }
    }
}

/// Authentication Error types.
#[derive(Debug)]
pub enum AuthError {
    InvalidCredentials,
    NotFound,
    HashError(HashError),
    Duplicate,
    Database(DbErr),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidCredentials => write!(f, "Invalid credentials"),
            AuthError::NotFound => write!(f, "User not found"),
            AuthError::HashError(e) => write!(f, "Password hash error: {}", e),
            AuthError::Duplicate => write!(f, "User already exists"),
            AuthError::Database(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl From<DbErr> for AuthError {
    fn from(e: DbErr) -> Self {
        if is_unique_violation(&e) {
            AuthError::Duplicate
        } else {
            AuthError::Database(e)
        }
    }
}

impl From<HashError> for AuthError {
    fn from(e: HashError) -> Self {
        AuthError::HashError(e)
    }
}

impl std::error::Error for AuthError {}

/// Invoice error type.
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

/// Payment creation error.
#[derive(Debug)]
pub enum PaymentCreateError {
    SameAccount,
    Duplicate,
    Database(DbErr),
    Posting(PostingError),
    Invoice(InvoiceError),
}

impl std::fmt::Display for PaymentCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentCreateError::SameAccount => {
                write!(f, "From and to accounts must differ")
            }
            PaymentCreateError::Duplicate => write!(f, "Duplicate record"),
            PaymentCreateError::Database(e) => write!(f, "Database error: {e}"),
            PaymentCreateError::Posting(e) => write!(f, "Posting failed: {e}"),
            PaymentCreateError::Invoice(e) => write!(f, "Invoice error: {e}"),
        }
    }
}

impl From<DbErr> for PaymentCreateError {
    fn from(e: DbErr) -> Self {
        if is_unique_violation(&e) {
            PaymentCreateError::Duplicate
        } else {
            PaymentCreateError::Database(e)
        }
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
            InvoiceError::Duplicate => PaymentCreateError::Duplicate,
            other => PaymentCreateError::Invoice(other),
        }
    }
}

impl std::error::Error for PaymentCreateError {}

/// Invoice Status error.
#[derive(Debug)]
pub enum InvoiceError {
    NotEditable,
    InvalidStatusTransition { from: String, to: String },
    NoLineItems,
    VoidWithPayments,
    AlreadyVoided,
    NotFound,
    Duplicate,
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
            InvoiceError::NotFound => write!(f, "Invoice not found"),
            InvoiceError::Duplicate => write!(f, "Duplicate invoice"),
            InvoiceError::ValidationError(msg) => write!(f, "{msg}"),
            InvoiceError::Posting(e) => write!(f, "Posting failed: {e}"),
            InvoiceError::Database(e) => write!(f, "Database error: {e}"),
        }
    }
}

impl From<DbErr> for InvoiceError {
    fn from(e: DbErr) -> Self {
        if is_unique_violation(&e) {
            InvoiceError::Duplicate
        } else {
            InvoiceError::Database(e)
        }
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
            AppError::NotFound => InvoiceError::NotFound,
            AppError::Duplicate => InvoiceError::Duplicate,
            AppError::InvalidArgument(msg) => InvoiceError::ValidationError(msg),
        }
    }
}

impl std::error::Error for InvoiceError {}

/// Claims error type.
#[derive(Debug)]
pub enum ClaimError {
    NotFound,
    InvalidInput(String),
    Duplicate,
    Database(DbErr),
    Posting(PostingError),
    InvalidStatus,
    NotOwner,
}

impl std::fmt::Display for ClaimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClaimError::NotFound => write!(f, "Claim not found"),
            ClaimError::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
            ClaimError::Duplicate => write!(f, "Duplicate claim"),
            ClaimError::Database(e) => write!(f, "Database error: {e}"),
            ClaimError::Posting(e) => write!(f, "Posting failed: {e}"),
            ClaimError::InvalidStatus => {
                write!(f, "Claim is not in a valid status for this action")
            }
            ClaimError::NotOwner => write!(f, "User is not the owner of this claim"),
        }
    }
}

impl From<DbErr> for ClaimError {
    fn from(e: DbErr) -> Self {
        ClaimError::Database(e)
    }
}

impl From<PostingError> for ClaimError {
    fn from(e: PostingError) -> Self {
        ClaimError::Posting(e)
    }
}

impl From<AppError> for ClaimError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::Database(db_err) => ClaimError::Database(db_err),
            AppError::NotFound => ClaimError::NotFound,
            AppError::Duplicate => ClaimError::Duplicate,
            AppError::InvalidArgument(msg) => ClaimError::InvalidInput(msg),
        }
    }
}

impl std::error::Error for ClaimError {}
