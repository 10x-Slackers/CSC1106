//! Module entry file for the claim model folder.
//! Connects the claim-related files together & re-exports.
//!
//! Authors: Teo Kai Wen

mod repository;
mod stats;
mod types;

pub use repository::title_map_by_ids;
pub use stats::ClaimStats;
pub use types::{Claim, ClaimFilter, ClaimForm};
