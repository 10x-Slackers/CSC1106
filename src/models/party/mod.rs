//! Module entry point for the party model folder.
//! Connects the other party-related files together & re-exports.
//!
//! Authors: Tan Yong Meng

mod repository;
mod stats;
mod types;

pub use stats::PartyStats;
pub use types::Party;
