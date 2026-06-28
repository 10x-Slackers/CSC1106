use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    Order, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};

use crate::entity::journal_entry::SourceDocument;
use crate::entity::journal_entry_line::EntrySide;
use crate::entity::party as party_entity;
use crate::entity::payment as payment_entity;
use crate::entity::payment::PaymentDirection;
use crate::entity::user as user_entity;
use crate::models::error::{AppError, PaymentCreateError};
use crate::models::posting::{JournalEntryLineInput, PostingService};

/// A payment with enrichment (party name, created-by name).
#[derive(Serialize)]
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
        match s.trim().to_uppercase().as_str() {
            "IN" => Some(PaymentDirection::In),
            "OUT" => Some(PaymentDirection::Out),
            _ => None,
        }
    }
}

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

impl Payment {
    async fn find_model_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<payment_entity::Model>, AppError> {
        payment_entity::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(AppError::from)
    }

    /// Load a payment with enriched details.
    pub async fn find_with_linked(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<PaymentDetail>, AppError> {
        let payment = match Self::find_model_by_id(db, id).await? {
            Some(m) => Payment::from(m),
            None => return Ok(None),
        };

        let party_name = match payment.party_id {
            Some(pid) => party_entity::Entity::find_by_id(pid)
                .one(db)
                .await?
                .map(|p| p.name),
            None => None,
        };

        let created_by_name = user_entity::Entity::find_by_id(payment.created_by_user_id)
            .one(db)
            .await?
            .map(|u| u.name);

        Ok(Some(PaymentDetail {
            id: payment.id,
            payment_direction: payment.payment_direction,
            payment_date: payment.payment_date,
            party_name,
            amount: payment.amount,
            remarks: payment.remarks,
            created_at: payment.created_at,
            created_by_name,
        }))
    }

    /// List payments with optional filters.
    /// `q` matches remarks (case-insensitive LIKE).
    pub async fn list(
        db: &DatabaseConnection,
        q: Option<&str>,
        direction: Option<PaymentDirection>,
        party_id: Option<i32>,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Result<Vec<Payment>, AppError> {
        let mut conditions = Condition::all();

        if let Some(query) = q {
            let query = query.trim();
            if !query.is_empty() {
                let escaped = query
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_");
                let pattern = format!("%{}%", escaped);
                conditions = conditions.add(payment_entity::Column::Remarks.like(&pattern));
            }
        }

        if let Some(dir) = direction {
            conditions = conditions.add(payment_entity::Column::PaymentDirection.eq(dir));
        }

        if let Some(pid) = party_id {
            conditions = conditions.add(payment_entity::Column::PartyId.eq(pid));
        }

        if let Some(from) = from_date {
            conditions = conditions.add(payment_entity::Column::PaymentDate.gte(from));
        }
        if let Some(to) = to_date {
            conditions = conditions.add(payment_entity::Column::PaymentDate.lte(to));
        }

        let payments = payment_entity::Entity::find()
            .filter(conditions)
            .order_by(payment_entity::Column::CreatedAt, Order::Desc)
            .all(db)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Payment::from)
            .collect();

        Ok(payments)
    }

    /// Create a payment and post the linked balanced journal entry.
    pub async fn create<C: ConnectionTrait>(
        db: &C,
        payment: Payment,
        from_account_id: i32,
        to_account_id: i32,
    ) -> Result<Payment, PaymentCreateError> {
        if from_account_id == to_account_id {
            return Err(PaymentCreateError::SameAccount);
        }

        let model = payment_entity::ActiveModel {
            invoice_id: Set(payment.invoice_id),
            party_id: Set(payment.party_id),
            created_by_user_id: Set(payment.created_by_user_id),
            payment_direction: Set(payment.payment_direction),
            amount: Set(payment.amount),
            payment_date: Set(payment.payment_date),
            remarks: Set(payment.remarks),
            ..Default::default()
        }
        .insert(db)
        .await?;

        let payment = Payment::from(model);

        let lines = vec![
            JournalEntryLineInput {
                account_id: to_account_id,
                entry_side: EntrySide::Debit,
                amount: payment.amount,
                description: None,
            },
            JournalEntryLineInput {
                account_id: from_account_id,
                entry_side: EntrySide::Credit,
                amount: payment.amount,
                description: None,
            },
        ];
        let source = SourceDocument::Payment {
            payment_id: payment.id,
        };
        PostingService::post_entry_in(db, lines, source, payment.created_by_user_id).await?;

        Ok(payment)
    }

    /// Sum of all payment amounts for a given party.
    pub async fn total_for_party(
        db: &DatabaseConnection,
        party_id: i32,
    ) -> Result<Decimal, AppError> {
        use sea_orm::sea_query::Expr;
        let total: Option<Decimal> = payment_entity::Entity::find()
            .filter(payment_entity::Column::PartyId.eq(party_id))
            .select_only()
            .expr(Expr::col(payment_entity::Column::Amount).sum())
            .into_tuple()
            .one(db)
            .await?;
        Ok(total.unwrap_or(Decimal::ZERO))
    }

    /// Most recent payments for a given party.
    pub async fn recent_for_party(
        db: &DatabaseConnection,
        party_id: i32,
        limit: u64,
    ) -> Result<Vec<Payment>, AppError> {
        let payments = payment_entity::Entity::find()
            .filter(payment_entity::Column::PartyId.eq(party_id))
            .order_by_desc(payment_entity::Column::CreatedAt)
            .limit(limit)
            .all(db)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Payment::from)
            .collect();

        Ok(payments)
    }
}
