//! Module entry point for the user model folder.
//! Connects the other user-related files together & re-exports.
//!
//! Authors: Tan Yong Meng

mod repository;
mod stats;
mod types;

pub use repository::{name_by_id, name_map_by_ids};
pub use stats::UserStats;
pub use types::User;
