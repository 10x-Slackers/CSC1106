use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::entity::journal_entry::SourceDocument;
use crate::entity::journal_entry_line::EntrySide;
use crate::entity::payment as payment_entity;
use crate::entity::payment::PaymentDirection;
use crate::models::error::{AppError, PaymentCreateError};
use crate::models::invoice::Invoice;
use crate::models::posting::{JournalEntryLineInput, PostingService};
use crate::models::util::{PER_PAGE, clamp_pagination, like_pattern};

use super::types::{Payment, PaymentDetail};

impl Payment {
    /// Load a payment with enriched details using a LEFT JOIN for party + a separate user query.
    pub async fn find_with_linked(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<PaymentDetail>, AppError> {
        use crate::entity::party as party_entity;
        use crate::entity::user as user_entity;

        let row = payment_entity::Entity::find_by_id(id)
            .find_also_related(party_entity::Entity)
            .one(db)
            .await?;

        match row {
            None => Ok(None),
            Some((payment, party)) => {
                let created_by_name = user_entity::Entity::find_by_id(payment.created_by_user_id)
                    .one(db)
                    .await?
                    .map(|u| u.name);

                Ok(Some(PaymentDetail {
                    id: payment.id,
                    payment_direction: payment.payment_direction,
                    payment_date: payment.payment_date,
                    party_name: party.map(|p| p.name),
                    amount: payment.amount,
                    remarks: payment.remarks,
                    created_at: payment.created_at,
                    created_by_name,
                }))
            }
        }
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
        page: u32,
    ) -> Result<(Vec<Payment>, u32, u32), AppError> {
        let mut conditions = Condition::all();

        if let Some(query) = q
            && let Some(pattern) = like_pattern(query)
        {
            conditions = conditions.add(payment_entity::Column::Remarks.like(&pattern));
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

        let paginator = payment_entity::Entity::find()
            .filter(conditions)
            .order_by(payment_entity::Column::CreatedAt, Order::Desc)
            .paginate(db, PER_PAGE);
        let num_pages = paginator.num_pages().await.map_err(AppError::from)?;
        let (total_pages, page) = clamp_pagination(page, num_pages);
        let payments = paginator
            .fetch_page((page - 1) as u64)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Payment::from)
            .collect();

        Ok((payments, total_pages, page))
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

        // Recompute invoice status if this payment is linked to an invoice
        if let Some(invoice_id) = payment.invoice_id {
            Invoice::recompute_status_for(db, invoice_id)
                .await
                .map_err(PaymentCreateError::from)?;
        }

        Ok(payment)
    }

    /// Sum of all payment amounts for a given party using SQL SUM.
    pub async fn total_for_party(
        db: &DatabaseConnection,
        party_id: i32,
    ) -> Result<Decimal, AppError> {
        let result: Option<(Option<Decimal>,)> = payment_entity::Entity::find()
            .select_only()
            .column_as(payment_entity::Column::Amount.sum(), "total")
            .filter(payment_entity::Column::PartyId.eq(party_id))
            .into_tuple()
            .one(db)
            .await?;
        Ok(result.and_then(|(v,)| v).unwrap_or(Decimal::ZERO))
    }

    /// Sum of all payment amounts for a given invoice using SQL SUM.
    pub async fn total_for_invoice<C: ConnectionTrait>(
        db: &C,
        invoice_id: i32,
    ) -> Result<Decimal, AppError> {
        let result: Option<(Option<Decimal>,)> = payment_entity::Entity::find()
            .select_only()
            .column_as(payment_entity::Column::Amount.sum(), "total")
            .filter(payment_entity::Column::InvoiceId.eq(invoice_id))
            .into_tuple()
            .one(db)
            .await?;
        Ok(result.and_then(|(v,)| v).unwrap_or(Decimal::ZERO))
    }

    /// List all payments linked to an invoice, newest first.
    pub async fn list_for_invoice(
        db: &DatabaseConnection,
        invoice_id: i32,
    ) -> Result<Vec<Payment>, AppError> {
        let payments = payment_entity::Entity::find()
            .filter(payment_entity::Column::InvoiceId.eq(invoice_id))
            .order_by_desc(payment_entity::Column::PaymentDate)
            .all(db)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Payment::from)
            .collect();
        Ok(payments)
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
