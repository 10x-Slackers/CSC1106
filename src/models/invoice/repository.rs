use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder,
};

use crate::entity::invoice as invoice_entity;
use crate::entity::invoice::InvoiceStatus;
use crate::entity::invoice_line_item as line_item_entity;
use crate::entity::party as party_entity;
use crate::models::error::{AppError, InvoiceError};
use crate::models::util::like_pattern;

use super::calc::grand_total;
use super::types::{Invoice, InvoiceLineItem};

impl Invoice {
    /// Load the grand total by summing the line items.
    pub(super) async fn load_grand_total<C: ConnectionTrait>(
        &self,
        db: &C,
    ) -> Result<Decimal, DbErr> {
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
    pub(super) async fn reload<C: ConnectionTrait>(
        db: &C,
        id: i32,
    ) -> Result<Invoice, InvoiceError> {
        let inv = invoice_entity::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| {
                InvoiceError::Database(sea_orm::DbErr::RecordNotFound("invoice".into()))
            })?;
        Ok(Invoice::from(inv))
    }

    async fn find_model_by_id<C: ConnectionTrait>(
        db: &C,
        id: i32,
    ) -> Result<Option<invoice_entity::Model>, AppError> {
        invoice_entity::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(AppError::from)
    }

    /// Load an invoice with its line items and creator name.
    pub async fn find_by_id<C: ConnectionTrait>(
        db: &C,
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
    pub async fn list<C: ConnectionTrait>(
        db: &C,
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

    /// Count invoices for a given party.
    pub async fn count_for_party<C: ConnectionTrait>(db: &C, party_id: i32) -> Result<u64, DbErr> {
        invoice_entity::Entity::find()
            .filter(invoice_entity::Column::PartyId.eq(party_id))
            .count(db)
            .await
    }
}
