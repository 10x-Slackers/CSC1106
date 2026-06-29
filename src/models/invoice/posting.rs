use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, DbErr,
    EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};

use crate::entity::invoice as invoice_entity;
use crate::entity::invoice::InvoiceStatus;
use crate::entity::invoice_line_item as line_item_entity;
use crate::entity::journal_entry::SourceDocument;
use crate::entity::journal_entry_line::EntrySide;
use crate::entity::party as party_entity;
use crate::models::error::{AppError, InvoiceError};
use crate::models::party::Party;
use crate::models::posting::{JournalEntryLineInput, PostingService};
use crate::models::util::{is_unique_violation, like_pattern};

use super::calc::grand_total;
use super::types::{Invoice, InvoiceLineItem, LineItemInput};

impl Invoice {
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
            updated_at: sea_orm::ActiveValue::NotSet, // set by ActiveModelBehavior::before_save
        }
    }

    /// Load the grand total by summing the line items.
    async fn load_grand_total<C: ConnectionTrait>(&self, db: &C) -> Result<Decimal, DbErr> {
        let items = line_item_entity::Entity::find()
            .filter(line_item_entity::Column::InvoiceId.eq(self.id))
            .all(db)
            .await?
            .into_iter()
            .map(InvoiceLineItem::from)
            .collect::<Vec<_>>();
        Ok(grand_total(&items))
    }

    /// Reload the invoice from the database, returning an error if not found.
    async fn reload<C: ConnectionTrait>(db: &C, id: i32) -> Result<Invoice, InvoiceError> {
        let inv = invoice_entity::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| {
                InvoiceError::Database(sea_orm::DbErr::RecordNotFound("invoice".into()))
            })?;
        Ok(Invoice::from(inv))
    }

    async fn find_model_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<invoice_entity::Model>, AppError> {
        invoice_entity::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(AppError::from)
    }

    /// Load an invoice with its line items and creator name.
    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<(Invoice, Vec<InvoiceLineItem>)>, AppError> {
        let inv = match Self::find_model_by_id(db, id).await? {
            Some(m) => m,
            None => return Ok(None),
        };
        let items = line_item_entity::Entity::find()
            .filter(line_item_entity::Column::InvoiceId.eq(id))
            .order_by(line_item_entity::Column::Id, Order::Asc)
            .all(db)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(InvoiceLineItem::from)
            .collect();

        let mut invoice = Invoice::from(inv);
        invoice.created_by_name =
            crate::models::user::name_by_id(db, invoice.created_by_user_id).await?;
        Ok(Some((invoice, items)))
    }

    /// List invoices with optional filters and staff scoping.
    #[allow(clippy::too_many_arguments)]
    pub async fn list(
        db: &DatabaseConnection,
        q: Option<&str>,
        status: Option<InvoiceStatus>,
        party_id: Option<i32>,
        from_issue: Option<NaiveDate>,
        to_issue: Option<NaiveDate>,
        from_due: Option<NaiveDate>,
        to_due: Option<NaiveDate>,
        viewer_user_id: i32,
        viewer_role: &crate::entity::user::Role,
    ) -> Result<Vec<Invoice>, AppError> {
        let mut conditions = Condition::all();

        if let Some(query) = q
            && let Some(pattern) = like_pattern(query)
        {
            conditions = conditions.add(invoice_entity::Column::InvoiceNo.like(&pattern));
        }

        if let Some(s) = status {
            conditions = conditions.add(invoice_entity::Column::Status.eq(s));
        }

        if let Some(pid) = party_id {
            conditions = conditions.add(invoice_entity::Column::PartyId.eq(pid));
        }

        if let Some(from) = from_issue {
            conditions = conditions.add(invoice_entity::Column::IssueDate.gte(from));
        }
        if let Some(to) = to_issue {
            conditions = conditions.add(invoice_entity::Column::IssueDate.lte(to));
        }
        if let Some(from) = from_due {
            conditions = conditions.add(invoice_entity::Column::DueDate.gte(from));
        }
        if let Some(to) = to_due {
            conditions = conditions.add(invoice_entity::Column::DueDate.lte(to));
        }

        // Staff scoping
        if *viewer_role == crate::entity::user::Role::Staff {
            conditions = conditions.add(invoice_entity::Column::CreatedByUserId.eq(viewer_user_id));
        }

        let invoices = invoice_entity::Entity::find()
            .filter(conditions)
            .order_by(invoice_entity::Column::IssueDate, Order::Desc)
            .order_by(invoice_entity::Column::Id, Order::Desc)
            .all(db)
            .await
            .map_err(AppError::from)?;

        if invoices.is_empty() {
            return Ok(vec![]);
        }

        // Enrich with party names and compute grand totals
        let party_ids: Vec<i32> = invoices.iter().map(|i| i.party_id).collect();
        let parties = party_entity::Entity::find()
            .filter(party_entity::Column::Id.is_in(party_ids))
            .all(db)
            .await
            .map_err(AppError::from)?;
        let party_map: std::collections::HashMap<i32, String> =
            parties.into_iter().map(|p| (p.id, p.name)).collect();

        let invoice_ids: Vec<i32> = invoices.iter().map(|i| i.id).collect();
        let all_items = line_item_entity::Entity::find()
            .filter(line_item_entity::Column::InvoiceId.is_in(invoice_ids))
            .all(db)
            .await
            .map_err(AppError::from)?;

        let mut items_by_invoice: std::collections::HashMap<i32, Vec<InvoiceLineItem>> =
            std::collections::HashMap::new();
        for item in all_items {
            items_by_invoice
                .entry(item.invoice_id)
                .or_default()
                .push(InvoiceLineItem::from(item));
        }

        let result = invoices
            .into_iter()
            .map(|m| {
                let mut inv = Invoice::from(m);
                inv.party_name = party_map.get(&inv.party_id).cloned();
                if let Some(items) = items_by_invoice.get(&inv.id) {
                    inv.grand_total = grand_total(items);
                }
                inv
            })
            .collect();

        Ok(result)
    }

    /// Generate the next invoice number: INV-YYYYMM-XXXX
    async fn generate_invoice_no<C: ConnectionTrait>(
        db: &C,
        issue_date: NaiveDate,
    ) -> Result<String, DbErr> {
        let prefix = format!("INV-{}", issue_date.format("%Y%m"));
        let count: u64 = invoice_entity::Entity::find()
            .filter(invoice_entity::Column::InvoiceNo.starts_with(&prefix))
            .count(db)
            .await?;
        Ok(format!("{}-{:04}", prefix, count + 1))
    }

    /// Create a new draft invoice with auto-generated number and line items.
    pub async fn create(
        db: &DatabaseConnection,
        party_id: i32,
        created_by_user_id: i32,
        issue_date: NaiveDate,
        due_date: NaiveDate,
        items: Vec<LineItemInput>,
    ) -> Result<Invoice, InvoiceError> {
        // Validate
        if due_date < issue_date {
            return Err(InvoiceError::ValidationError(
                "Due date cannot be before issue date".into(),
            ));
        }
        if items.is_empty() {
            return Err(InvoiceError::NoLineItems);
        }
        for item in &items {
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

        // Check party exists and is active
        let party = Party::find_by_id(db, party_id).await?.ok_or_else(|| {
            InvoiceError::Database(sea_orm::DbErr::RecordNotFound("party".into()))
        })?;
        if party.status != crate::entity::party::PartyStatus::Active {
            return Err(InvoiceError::ValidationError(
                "Selected party is not active".into(),
            ));
        }

        let txn = db.begin().await?;

        // Generate invoice number with retry on unique violation
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

        let line_ams: Vec<line_item_entity::ActiveModel> = items
            .into_iter()
            .map(|item| line_item_entity::ActiveModel {
                invoice_id: Set(invoice_model.last_insert_id),
                description: Set(item.description.trim().into()),
                quantity: Set(item.quantity),
                unit_price: Set(item.unit_price),
                gst_rate: Set(item.gst_rate),
                ..Default::default()
            })
            .collect();

        line_item_entity::Entity::insert_many(line_ams)
            .exec(&txn)
            .await?;

        txn.commit().await?;

        Self::reload(db, invoice_model.last_insert_id).await
    }

    /// Update a draft invoice's dates and line items. Only allowed in `Draft` status.
    pub async fn update(
        &self,
        db: &DatabaseConnection,
        issue_date: NaiveDate,
        due_date: NaiveDate,
        items: Vec<LineItemInput>,
    ) -> Result<Invoice, InvoiceError> {
        if self.status != InvoiceStatus::Draft {
            return Err(InvoiceError::NotEditable);
        }
        if due_date < issue_date {
            return Err(InvoiceError::ValidationError(
                "Due date cannot be before issue date".into(),
            ));
        }
        if items.is_empty() {
            return Err(InvoiceError::NoLineItems);
        }
        for item in &items {
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

        // Check party is still active
        let party = Party::find_by_id(db, self.party_id).await?.ok_or_else(|| {
            InvoiceError::Database(sea_orm::DbErr::RecordNotFound("party".into()))
        })?;
        if party.status != crate::entity::party::PartyStatus::Active {
            return Err(InvoiceError::ValidationError(
                "Selected party is not active".into(),
            ));
        }

        let txn = db.begin().await?;

        // Delete existing line items
        line_item_entity::Entity::delete_many()
            .filter(line_item_entity::Column::InvoiceId.eq(self.id))
            .exec(&txn)
            .await?;

        // Insert new line items
        let line_ams: Vec<line_item_entity::ActiveModel> = items
            .into_iter()
            .map(|item| line_item_entity::ActiveModel {
                invoice_id: Set(self.id),
                description: Set(item.description.trim().into()),
                quantity: Set(item.quantity),
                unit_price: Set(item.unit_price),
                gst_rate: Set(item.gst_rate),
                ..Default::default()
            })
            .collect();

        line_item_entity::Entity::insert_many(line_ams)
            .exec(&txn)
            .await?;

        // Update invoice dates
        let mut am = self.to_active_model();
        am.issue_date = Set(issue_date);
        am.due_date = Set(due_date);

        am.update(&txn).await?;

        txn.commit().await?;

        Self::reload(db, self.id).await
    }

    /// Post a draft invoice to the ledger (`Draft` → `Sent`).
    ///
    /// Posts DR Accounts Receivable, CR Sales Revenue for the invoice grand total.
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

        let total = current
            .load_grand_total(&txn)
            .await
            .map_err(InvoiceError::from)?;

        let (ar, sales) = crate::models::account::find_ar_and_sr(&txn).await?;

        let lines = vec![
            JournalEntryLineInput {
                account_id: ar.id,
                entry_side: EntrySide::Debit,
                amount: total,
                description: Some(format!("Invoice {}", current.invoice_no)),
            },
            JournalEntryLineInput {
                account_id: sales.id,
                entry_side: EntrySide::Credit,
                amount: total,
                description: Some(format!("Invoice {}", current.invoice_no)),
            },
        ];

        PostingService::post_entry_in(
            &txn,
            lines,
            SourceDocument::Invoice {
                invoice_id: current.id,
            },
            user_id,
        )
        .await?;

        let mut am = current.to_active_model();
        am.status = Set(InvoiceStatus::Sent);
        am.update(&txn).await?;

        txn.commit().await?;

        Self::reload(db, self.id).await
    }

    /// Void an invoice. `Draft` → `Voided` with no posting; `Sent` → `Voided`
    /// with a reversal entry (DR Sales Revenue, CR Accounts Receivable).
    /// Disallowed from `PartiallyPaid`, `Paid`, and `Voided`.
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
                let mut am = self.to_active_model();
                am.status = Set(InvoiceStatus::Voided);
                am.update(&txn).await?;
                txn.commit().await?;
                Self::reload(db, self.id).await
            }
            InvoiceStatus::Sent => {
                // Reversal entry: DR Sales Revenue, CR Accounts Receivable
                let txn = db.begin().await?;

                let current = Self::reload(&txn, self.id).await?;
                if current.status != InvoiceStatus::Sent {
                    txn.rollback().await?;
                    return Err(InvoiceError::InvalidStatusTransition {
                        from: current.status.to_string(),
                        to: InvoiceStatus::Voided.to_string(),
                    });
                }

                let total = current
                    .load_grand_total(&txn)
                    .await
                    .map_err(InvoiceError::from)?;

                let (ar, sales) = crate::models::account::find_ar_and_sr(&txn).await?;

                let lines = vec![
                    JournalEntryLineInput {
                        account_id: sales.id,
                        entry_side: EntrySide::Debit,
                        amount: total,
                        description: Some(format!("Void invoice {}", current.invoice_no)),
                    },
                    JournalEntryLineInput {
                        account_id: ar.id,
                        entry_side: EntrySide::Credit,
                        amount: total,
                        description: Some(format!("Void invoice {}", current.invoice_no)),
                    },
                ];

                PostingService::post_entry_in(
                    &txn,
                    lines,
                    SourceDocument::Invoice {
                        invoice_id: current.id,
                    },
                    user_id,
                )
                .await?;

                let mut am = current.to_active_model();
                am.status = Set(InvoiceStatus::Voided);
                am.update(&txn).await?;

                txn.commit().await?;

                Self::reload(db, self.id).await
            }
        }
    }

    /// Recompute status based on total paid amount.
    pub async fn recompute_status<C: ConnectionTrait>(
        &self,
        db: &C,
        total_paid: Decimal,
    ) -> Result<Invoice, InvoiceError> {
        match self.status {
            InvoiceStatus::Paid | InvoiceStatus::Voided => Ok(self.clone()),
            InvoiceStatus::Draft => Err(InvoiceError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "recompute".into(),
            }),
            InvoiceStatus::Sent | InvoiceStatus::PartiallyPaid => {
                let total = self.load_grand_total(db).await?;

                let new_status = if total_paid >= total {
                    InvoiceStatus::Paid
                } else if total_paid > Decimal::ZERO {
                    InvoiceStatus::PartiallyPaid
                } else {
                    InvoiceStatus::Sent
                };

                let mut am = self.to_active_model();
                am.status = Set(new_status);
                am.update(db).await?;

                Self::reload(db, self.id).await
            }
        }
    }

    /// Load an invoice by ID and recompute its status from cumulative payments.
    pub async fn recompute_status_for<C: ConnectionTrait>(
        db: &C,
        invoice_id: i32,
    ) -> Result<(), InvoiceError> {
        let model = invoice_entity::Entity::find_by_id(invoice_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                InvoiceError::Database(sea_orm::DbErr::RecordNotFound("invoice".into()))
            })?;
        let invoice = Invoice::from(model);

        let total_paid: Decimal = crate::entity::payment::Entity::find()
            .filter(crate::entity::payment::Column::InvoiceId.eq(invoice_id))
            .select_only()
            .expr(sea_orm::sea_query::Expr::col(crate::entity::payment::Column::Amount).sum())
            .into_tuple()
            .one(db)
            .await
            .map_err(InvoiceError::from)?
            .unwrap_or(Decimal::ZERO);

        invoice.recompute_status(db, total_paid).await?;
        Ok(())
    }
}
