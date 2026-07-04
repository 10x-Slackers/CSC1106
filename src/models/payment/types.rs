use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::entity::payment as payment_entity;
pub use crate::entity::payment::PaymentDirection;

/// A payment with its party name and creator's name attached,
/// for the single-payment display page.
#[derive(Clone, Serialize)]
pub struct PaymentDetail {
    pub id: i32,
    pub payment_direction: PaymentDirection,
    pub payment_date: NaiveDate,
    pub party_name: Option<String>,
    pub amount: Decimal,
    pub remarks: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub created_by_name: Option<String>,
}

impl PaymentDirection {
    /// Parse a payment direction string into a `PaymentDirection`.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("in") {
            Some(PaymentDirection::In)
        } else if s.eq_ignore_ascii_case("out") {
            Some(PaymentDirection::Out)
        } else {
            None
        }
    }
}

/// The plain payment record (raw `party_id`/`created_by_user_id`); see
/// `PaymentDetail` for the version with names resolved for display.
#[derive(Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: i32,
    pub invoice_id: Option<i32>,
    pub party_id: Option<i32>,
    pub created_by_user_id: i32,
    pub payment_direction: PaymentDirection,
    pub amount: Decimal,
    pub payment_date: NaiveDate,
    pub remarks: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

impl From<payment_entity::Model> for Payment {
    fn from(m: payment_entity::Model) -> Self {
        Payment {
            id: m.id,
            invoice_id: m.invoice_id,
            party_id: m.party_id,
            created_by_user_id: m.created_by_user_id,
            payment_direction: m.payment_direction,
            amount: m.amount,
            payment_date: m.payment_date,
            remarks: m.remarks,
            created_at: m.created_at,
        }
    }
}
