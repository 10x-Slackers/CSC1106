//! PDF generation (approach 1: pure-Rust `genpdf` builder).
//!
//! [`builder`] holds document-agnostic building blocks; each document type has
//! its own layout module that composes them from real domain data.

pub mod audit;
pub mod balance_sheet;
mod builder;
pub mod gst;
pub mod income_statement;
pub mod invoice;

pub use audit::render_audit;
pub use balance_sheet::render_balance_sheet;
pub use gst::render_gst;
pub use income_statement::render_income_statement;
pub use invoice::render_invoice;

/// Errors raised while generating a PDF.
#[derive(Debug)]
pub enum PdfError {
    /// Font data could not be parsed by the PDF engine.
    Font(String),
    /// The document failed to render.
    Render(String),
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfError::Font(msg) => write!(f, "Invalid font data: {msg}"),
            PdfError::Render(msg) => write!(f, "Failed to render PDF: {msg}"),
        }
    }
}

impl std::error::Error for PdfError {}
