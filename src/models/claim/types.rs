//! Defines the application-level data structures for claims.
//!
//! Authors: Teo Kai Wen

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::entity::claim as claim_entity;

/// Application-level claim model converted from the database entity.
#[derive(Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: i32,
    pub submitted_by_user_id: i32,
    pub reviewed_by_user_id: Option<i32>,
    pub category_id: i32,
    pub title: String,
    pub description: String,
    pub amount: Decimal,
    pub purchase_date: NaiveDate,
    pub status: String,
    pub rejection_reason: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// Convert the SeaORM claim entity into the application-level claim model.
impl From<claim_entity::Model> for Claim {
    fn from(m: claim_entity::Model) -> Self {
        Claim {
            id: m.id,
            submitted_by_user_id: m.submitted_by_user_id,
            reviewed_by_user_id: m.reviewed_by_user_id,
            category_id: m.category_id,
            title: m.title,
            description: m.description,
            amount: m.amount,
            purchase_date: m.purchase_date,
            status: m.status.to_string(),
            rejection_reason: m.rejection_reason,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

/// Claim detail data enriched with names for the detail view.
#[derive(Clone, Serialize)]
pub struct ClaimDetail {
    pub id: i32,
    pub submitted_by_user_id: i32,
    pub reviewed_by_user_id: Option<i32>,
    pub category_id: i32,
    pub title: String,
    pub description: String,
    pub amount: Decimal,
    pub purchase_date: NaiveDate,
    pub status: String,
    pub rejection_reason: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub submitter_name: String,
    pub reviewer_name: Option<String>,
    pub category_name: String,
}

/// Form input for creating a new claim.
#[derive(Clone, Deserialize)]
pub struct ClaimForm {
    pub title: String,
    pub category_id: i32,
    pub description: String,
    pub amount: String,
    pub purchase_date: String,
}

/// Query-string filter parameters for the claim list.
#[derive(Clone, Deserialize, Serialize, Default)]
pub struct ClaimFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitted_by_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
}

/// Row for the claim list table.
#[derive(Clone, Serialize)]
pub struct ClaimRow {
    pub id: i32,
    pub title: String,
    pub submitter_name: String,
    pub category_name: String,
    pub amount: Decimal,
    pub purchase_date: NaiveDate,
    pub status: String,
}
