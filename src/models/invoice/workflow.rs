use chrono::NaiveDate;
use futures::TryFutureExt;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, Set, TransactionTrait,
};

use crate::entity::invoice as invoice_entity;
use crate::entity::invoice::InvoiceStatus;
use crate::entity::invoice_line_item as line_item_entity;
use crate::entity::journal_entry::SourceDocument;
use crate::entity::journal_entry_line::EntrySide;
use crate::entity::party::{PartyStatus, PartyType};
use crate::models::account::find_ar_and_sr;
use crate::models::error::InvoiceError;
use crate::models::journal_entry::{JournalEntry, JournalEntryLineInput};
use crate::models::party::Party;
use crate::models::payment::Payment;
use crate::models::util::is_unique_violation;

use super::calc::grand_total;
use super::types::{Invoice, InvoiceLineItem, LineItemInput};

impl Invoice {
    /// Convert the invoice model into database struct
    fn to_active_model(&self) -> invoice_entity::ActiveModel {
        invoice_entity::ActiveModel {
            id: Set(self.id),
            party_id: Set(self.party_id),
            created_by_user_id: Set(self.created_by_user_id),
            invoice_no: Set(self.invoice_no.clone()),
            issue_date: Set(self.issue_date),
            due_date: Set(self.due_date),
            status: Set(self.status.clone()),
            created_at: Set(self.created_at),
            updated_at: sea_orm::ActiveValue::NotSet, // auto updated by SeaORM before_save hook
        }
    }

    fn validate_inputs(
        issue_date: NaiveDate,
        due_date: NaiveDate,
        items: &[LineItemInput],
    ) -> Result<(), InvoiceError> {
        if due_date < issue_date {
            return Err(InvoiceError::ValidationError(
                "Due date cannot be before issue date".into(),
            ));
        }
        if items.is_empty() {
            return Err(InvoiceError::NoLineItems);
        }
        for item in items {
            if item.description.trim().is_empty() {
                return Err(InvoiceError::ValidationError(
                    "Line item description cannot be empty".into(),
                ));
            }
            if item.quantity <= 0 {
                return Err(InvoiceError::ValidationError(
                    "Line item quantity must be positive".into(),
                ));
            }
            if item.unit_price < Decimal::ZERO {
                return Err(InvoiceError::ValidationError(
                    "Line item unit price cannot be negative".into(),
                ));
            }
        }
        Ok(())
    }

    fn line_items_to_active(
        invoice_id: i32,
        items: Vec<LineItemInput>,
    ) -> Vec<line_item_entity::ActiveModel> {
        items
            .into_iter()
            .map(|item| line_item_entity::ActiveModel {
                invoice_id: Set(invoice_id),
                description: Set(item.description.trim().into()),
                quantity: Set(item.quantity),
                unit_price: Set(item.unit_price),
                gst_rate: Set(item.gst_rate),
                ..Default::default()
            })
            .collect()
    }

    async fn require_active_party(
        db: &DatabaseConnection,
        party_id: i32,
    ) -> Result<(), InvoiceError> {
        let party = Party::find_by_id(db, party_id)
            .await?
            .ok_or_else(|| InvoiceError::NotFound)?;
        if party.status != PartyStatus::Active {
            return Err(InvoiceError::ValidationError(
                "Selected party is not active".into(),
            ));
        }
        if party.party_type != PartyType::Customer {
            return Err(InvoiceError::ValidationError(
                "Invoices can only be created for customers".into(),
            ));
        }
        Ok(())
    }

    async fn generate_invoice_no<C: ConnectionTrait>(
        db: &C,
        issue_date: NaiveDate,
    ) -> Result<String, InvoiceError> {
        let prefix = format!("INV-{}", issue_date.format("%Y%m"));
        let count: u64 = invoice_entity::Entity::find()
            .filter(invoice_entity::Column::InvoiceNo.starts_with(&prefix))
            .count(db)
            .await?;
        Ok(format!("{}-{:04}", prefix, count + 1))
    }

    async fn post_invoice_entry(
        txn: &impl ConnectionTrait,
        current: &Invoice,
        new_status: InvoiceStatus,
        description: &str,
        debit_ar: bool,
        user_id: i32,
    ) -> Result<(), InvoiceError> {
        let total = current.load_grand_total(txn).await?;

        let (ar, sales) = find_ar_and_sr(txn).await?;

        let (ar_side, sales_side) = if debit_ar {
            (EntrySide::Debit, EntrySide::Credit)
        } else {
            (EntrySide::Credit, EntrySide::Debit)
        };

        let lines = vec![
            JournalEntryLineInput {
                account_id: ar.id,
                entry_side: ar_side,
                amount: total,
                description: Some(description.to_string()),
            },
            JournalEntryLineInput {
                account_id: sales.id,
                entry_side: sales_side,
                amount: total,
                description: Some(description.to_string()),
            },
        ];

        JournalEntry::create(
            txn,
            lines,
            SourceDocument::Invoice {
                invoice_id: current.id,
            },
            user_id,
        )
        .await?;

        let mut am = current.to_active_model();
        am.status = Set(new_status);
        am.update(txn).await?;
        Ok(())
    }

    /// creates new invoice in Draft Status
    pub async fn create(
        db: &DatabaseConnection,
        party_id: i32,
        created_by_user_id: i32,
        issue_date: NaiveDate,
        due_date: NaiveDate,
        items: Vec<LineItemInput>,
    ) -> Result<Invoice, InvoiceError> {
        Self::validate_inputs(issue_date, due_date, &items)?;

        Self::require_active_party(db, party_id).await?;

        let txn = db.begin().await?;

        let mut invoice_no = Self::generate_invoice_no(&txn, issue_date).await?;
        let mut retries = 0;
        let max_retries = 5;

        let invoice_model = loop {
            let am = invoice_entity::ActiveModel {
                party_id: Set(party_id),
                created_by_user_id: Set(created_by_user_id),
                invoice_no: Set(invoice_no.clone()),
                issue_date: Set(issue_date),
                due_date: Set(due_date),
                status: Set(InvoiceStatus::Draft),
                ..Default::default()
            };

            match invoice_entity::Entity::insert(am).exec(&txn).await {
                Ok(res) => break res,
                Err(ref e) if is_unique_violation(e) && retries < max_retries => {
                    retries += 1;
                    invoice_no = Self::generate_invoice_no(&txn, issue_date).await?;
                }
                Err(e) => return Err(InvoiceError::Database(e)),
            }
        };

        let line_ams: Vec<line_item_entity::ActiveModel> =
            Self::line_items_to_active(invoice_model.last_insert_id, items);

        line_item_entity::Entity::insert_many(line_ams)
            .exec(&txn)
            .await?;

        txn.commit().await?;

        Self::reload(db, invoice_model.last_insert_id).await
    }

    /// updates the invoice if it is in Draft status, otherwise returns an error
    pub async fn update(
        &self,
        db: &DatabaseConnection,
        issue_date: NaiveDate,
        due_date: NaiveDate,
        items: Vec<LineItemInput>,
    ) -> Result<Invoice, InvoiceError> {
        Self::validate_inputs(issue_date, due_date, &items)?;

        Self::require_active_party(db, self.party_id).await?;

        let txn = db.begin().await?;

        let current = Self::reload(&txn, self.id).await?;
        if current.status != InvoiceStatus::Draft {
            txn.rollback().await?;
            return Err(InvoiceError::NotEditable);
        }

        line_item_entity::Entity::delete_many()
            .filter(line_item_entity::Column::InvoiceId.eq(self.id))
            .exec(&txn)
            .await?;

        let line_ams: Vec<line_item_entity::ActiveModel> =
            Self::line_items_to_active(self.id, items);

        line_item_entity::Entity::insert_many(line_ams)
            .exec(&txn)
            .await?;

        let mut am = current.to_active_model();
        am.issue_date = Set(issue_date);
        am.due_date = Set(due_date);

        am.update(&txn).await?;

        txn.commit().await?;

        Self::reload(db, self.id).await
    }

    /// sends the invoice, updates the status to Sent, and creates a journal entry for the invoice
    pub async fn send(
        &self,
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<Invoice, InvoiceError> {
        let txn = db.begin().await?;

        let current = Self::reload(&txn, self.id).await?;
        if current.status != InvoiceStatus::Draft {
            txn.rollback().await?;
            return Err(InvoiceError::InvalidStatusTransition {
                from: current.status.to_string(),
                to: InvoiceStatus::Sent.to_string(),
            });
        }

        let description = format!("Invoice {}", current.invoice_no);
        Self::post_invoice_entry(
            &txn,
            &current,
            InvoiceStatus::Sent,
            &description,
            true,
            user_id,
        )
        .await?;

        txn.commit().await?;

        Self::reload(db, self.id).await
    }

    /// voids the invoice and reverse the journal entry if the invoice is sent
    pub async fn void(
        &self,
        db: &DatabaseConnection,
        user_id: i32,
    ) -> Result<Invoice, InvoiceError> {
        match self.status {
            InvoiceStatus::Voided => Err(InvoiceError::AlreadyVoided),
            InvoiceStatus::PartiallyPaid | InvoiceStatus::Paid => {
                Err(InvoiceError::VoidWithPayments)
            }
            InvoiceStatus::Draft => {
                let txn = db.begin().await?;

                let current = Self::reload(&txn, self.id).await?;
                if current.status != InvoiceStatus::Draft {
                    txn.rollback().await?;
                    return Err(InvoiceError::InvalidStatusTransition {
                        from: current.status.to_string(),
                        to: InvoiceStatus::Voided.to_string(),
                    });
                }

                let mut am = current.to_active_model();
                am.status = Set(InvoiceStatus::Voided);
                am.update(&txn).await?;
                txn.commit().await?;
                Self::reload(db, self.id).await
            }
            InvoiceStatus::Sent => {
                let txn = db.begin().await?;

                let current = Self::reload(&txn, self.id).await?;
                if current.status != InvoiceStatus::Sent {
                    txn.rollback().await?;
                    return Err(InvoiceError::InvalidStatusTransition {
                        from: current.status.to_string(),
                        to: InvoiceStatus::Voided.to_string(),
                    });
                }

                let description = format!("Void invoice {}", current.invoice_no);
                Self::post_invoice_entry(
                    &txn,
                    &current,
                    InvoiceStatus::Voided,
                    &description,
                    false,
                    user_id,
                )
                .await?;

                txn.commit().await?;

                Self::reload(db, self.id).await
            }
        }
    }

    async fn apply_recomputed_status<C: ConnectionTrait>(
        &self,
        db: &C,
        total_paid: Decimal,
        grand_total: Decimal,
    ) -> Result<(), InvoiceError> {
        match self.status {
            InvoiceStatus::Paid | InvoiceStatus::Voided => Ok(()),
            InvoiceStatus::Draft => Err(InvoiceError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "recompute".into(),
            }),
            InvoiceStatus::Sent | InvoiceStatus::PartiallyPaid => {
                let new_status = if total_paid >= grand_total {
                    InvoiceStatus::Paid
                } else if total_paid > Decimal::ZERO {
                    InvoiceStatus::PartiallyPaid
                } else {
                    InvoiceStatus::Sent
                };

                let mut am = invoice_entity::ActiveModel {
                    id: Set(self.id),
                    ..Default::default()
                };
                am.status = Set(new_status);
                am.update(db).await?;
                Ok(())
            }
        }
    }

    pub async fn recompute_status_for<C: ConnectionTrait>(
        db: &C,
        invoice_id: i32,
    ) -> Result<(), InvoiceError> {
        let model = invoice_entity::Entity::find_by_id(invoice_id)
            .one(db)
            .await?
            .ok_or_else(|| InvoiceError::NotFound)?;
        let invoice = Invoice::from(model);

        let (items, total_paid) = {
            let items_fut = line_item_entity::Entity::find()
                .filter(line_item_entity::Column::InvoiceId.eq(invoice_id))
                .all(db)
                .map_err(InvoiceError::from);
            let tp_fut = Payment::total_for_invoice(db, invoice_id).map_err(InvoiceError::from);
            futures::try_join!(items_fut, tp_fut)?
        };
        let items: Vec<InvoiceLineItem> = items.into_iter().map(InvoiceLineItem::from).collect();
        let grand_total = grand_total(&items);

        invoice
            .apply_recomputed_status(db, total_paid, grand_total)
            .await?;
        Ok(())
    }
}
