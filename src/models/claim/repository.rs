use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    Order, PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};

use crate::entity::claim as claim_entity;
use crate::entity::claim::ClaimStatus;
use crate::entity::journal_entry::SourceDocument;
use crate::entity::journal_entry_line::EntrySide;
use crate::models::account::find_oe_and_ap;
use crate::models::claim_category::{
    find_name_by_id as category_name_by_id, name_map_by_ids as category_name_map_by_ids,
};
use crate::models::error::ClaimError;
use crate::models::posting::{JournalEntryLineInput, PostingService};
use crate::models::user::{name_by_id, name_map_by_ids};
use crate::models::util::{PER_PAGE, clamp_pagination, like_pattern};

use super::types::{Claim, ClaimDetail, ClaimFilter, ClaimForm, ClaimRow};

impl Claim {
    async fn find_model_by_id<C: ConnectionTrait>(
        db: &C,
        id: i32,
    ) -> Result<claim_entity::Model, ClaimError> {
        claim_entity::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(ClaimError::NotFound)
    }

    /// Load a claim with enriched details (submitter, reviewer, category names).
    pub async fn find_with_linked<C: ConnectionTrait>(
        db: &C,
        id: i32,
    ) -> Result<ClaimDetail, ClaimError> {
        let model = Self::find_model_by_id(db, id).await?;
        let claim = Claim::from(model);

        let submitter_name = name_by_id(db, claim.submitted_by_user_id)
            .await?
            .unwrap_or_default();

        let reviewer_name = match claim.reviewed_by_user_id {
            Some(uid) => name_by_id(db, uid).await?,
            None => None,
        };

        let category_name = category_name_by_id(db, claim.category_id)
            .await?
            .unwrap_or_default();

        Ok(ClaimDetail {
            id: claim.id,
            submitted_by_user_id: claim.submitted_by_user_id,
            reviewed_by_user_id: claim.reviewed_by_user_id,
            category_id: claim.category_id,
            title: claim.title,
            description: claim.description,
            amount: claim.amount,
            purchase_date: claim.purchase_date,
            status: claim.status,
            rejection_reason: claim.rejection_reason,
            created_at: claim.created_at,
            updated_at: claim.updated_at,
            submitter_name,
            reviewer_name,
            category_name,
        })
    }

    /// List claims with optional filters and pagination.
    pub async fn list<C: ConnectionTrait>(
        db: &C,
        filter: &ClaimFilter,
        scope_user_id: Option<i32>,
    ) -> Result<(Vec<ClaimRow>, u32, u32), ClaimError> {
        let mut conditions = Condition::all();

        if let Some(ref q) = filter.q
            && let Some(pattern) = like_pattern(q)
        {
            conditions = conditions.add(
                Condition::any()
                    .add(claim_entity::Column::Title.like(&pattern))
                    .add(claim_entity::Column::Description.like(&pattern)),
            );
        }

        if let Some(ref status_str) = filter.status {
            let status = match status_str.to_uppercase().as_str() {
                "PENDING" => Some(ClaimStatus::Pending),
                "APPROVED" => Some(ClaimStatus::Approved),
                "REJECTED" => Some(ClaimStatus::Rejected),
                _ => None,
            };
            if let Some(status) = status {
                conditions = conditions.add(claim_entity::Column::Status.eq(status));
            }
        }

        if let Some(cat_id) = filter.category_id {
            conditions = conditions.add(claim_entity::Column::CategoryId.eq(cat_id));
        }

        if let Some(scope) = scope_user_id {
            conditions = conditions.add(claim_entity::Column::SubmittedByUserId.eq(scope));
        } else if let Some(uid) = filter.submitted_by_user_id {
            conditions = conditions.add(claim_entity::Column::SubmittedByUserId.eq(uid));
        }

        if let Some(ref from_str) = filter.from
            && let Ok(from) = NaiveDate::parse_from_str(from_str, "%Y-%m-%d")
        {
            conditions = conditions.add(claim_entity::Column::PurchaseDate.gte(from));
        }

        if let Some(ref to_str) = filter.to
            && let Ok(to) = NaiveDate::parse_from_str(to_str, "%Y-%m-%d")
        {
            conditions = conditions.add(claim_entity::Column::PurchaseDate.lte(to));
        }

        let page = filter.page.unwrap_or(1).max(1);

        let paginator = claim_entity::Entity::find()
            .filter(conditions)
            .order_by(claim_entity::Column::CreatedAt, Order::Desc)
            .paginate(db, PER_PAGE);
        let num_pages = paginator.num_pages().await.map_err(ClaimError::from)?;
        let (total_pages, current_page) = clamp_pagination(page, num_pages);
        let models = paginator
            .fetch_page((current_page - 1) as u64)
            .await
            .map_err(ClaimError::from)?;

        let rows = if models.is_empty() {
            Vec::new()
        } else {
            let user_ids: Vec<i32> = models.iter().map(|m| m.submitted_by_user_id).collect();
            let user_map = name_map_by_ids(db, user_ids).await?;

            let category_ids: Vec<i32> = models.iter().map(|m| m.category_id).collect();
            let category_map = category_name_map_by_ids(db, category_ids).await?;

            models
                .into_iter()
                .map(|m| ClaimRow {
                    submitter_name: user_map
                        .get(&m.submitted_by_user_id)
                        .cloned()
                        .unwrap_or_default(),
                    category_name: category_map
                        .get(&m.category_id)
                        .cloned()
                        .unwrap_or_default(),
                    id: m.id,
                    title: m.title,
                    amount: m.amount,
                    purchase_date: m.purchase_date,
                    status: m.status.to_string(),
                })
                .collect()
        };

        Ok((rows, total_pages, current_page))
    }

    /// Create a new claim with Pending status.
    pub async fn create<C: ConnectionTrait>(
        db: &C,
        form: ClaimForm,
        submitted_by_user_id: i32,
    ) -> Result<claim_entity::Model, ClaimError> {
        let amount: Decimal = form.amount.parse().map_err(|_| {
            ClaimError::Database(sea_orm::DbErr::Custom(format!(
                "Invalid amount: {}",
                form.amount
            )))
        })?;
        let purchase_date =
            NaiveDate::parse_from_str(&form.purchase_date, "%Y-%m-%d").map_err(|_| {
                ClaimError::Database(sea_orm::DbErr::Custom(format!(
                    "Invalid purchase date: {}",
                    form.purchase_date
                )))
            })?;

        let model = claim_entity::ActiveModel {
            submitted_by_user_id: Set(submitted_by_user_id),
            reviewed_by_user_id: Set(None),
            category_id: Set(form.category_id),
            title: Set(form.title),
            description: Set(form.description),
            amount: Set(amount),
            purchase_date: Set(purchase_date),
            status: Set(ClaimStatus::Pending),
            rejection_reason: Set(None),
            ..Default::default()
        }
        .insert(db)
        .await?;

        Ok(model)
    }

    /// Approve a claim: Pending → Approved, posts DR Operating Expenses / CR Accounts Payable.
    pub async fn approve(
        db: &DatabaseConnection,
        id: i32,
        reviewer_id: i32,
    ) -> Result<claim_entity::Model, ClaimError> {
        let txn = db.begin().await?;

        let model = Self::find_model_by_id(&txn, id).await?;
        if model.status != ClaimStatus::Pending {
            txn.rollback().await?;
            return Err(ClaimError::InvalidStatus);
        }

        let (oe, ap) = find_oe_and_ap(&txn).await.map_err(ClaimError::from)?;

        let lines = vec![
            JournalEntryLineInput {
                account_id: oe.id,
                entry_side: EntrySide::Debit,
                amount: model.amount,
                description: Some(format!("Claim: {}", model.title)),
            },
            JournalEntryLineInput {
                account_id: ap.id,
                entry_side: EntrySide::Credit,
                amount: model.amount,
                description: Some(format!("Claim: {}", model.title)),
            },
        ];

        PostingService::post_entry_in(
            &txn,
            lines,
            SourceDocument::Claim { claim_id: id },
            reviewer_id,
        )
        .await?;

        let mut am: claim_entity::ActiveModel = model.into();
        am.status = Set(ClaimStatus::Approved);
        am.reviewed_by_user_id = Set(Some(reviewer_id));
        let updated = am.update(&txn).await?;

        txn.commit().await?;

        Ok(updated)
    }

    /// Reject a claim: Pending → Rejected, no posting.
    pub async fn reject(
        db: &DatabaseConnection,
        id: i32,
        reviewer_id: i32,
        reason: String,
    ) -> Result<claim_entity::Model, ClaimError> {
        let txn = db.begin().await?;

        let model = Self::find_model_by_id(&txn, id).await?;
        if model.status != ClaimStatus::Pending {
            txn.rollback().await?;
            return Err(ClaimError::InvalidStatus);
        }

        let mut am: claim_entity::ActiveModel = model.into();
        am.status = Set(ClaimStatus::Rejected);
        am.reviewed_by_user_id = Set(Some(reviewer_id));
        am.rejection_reason = Set(Some(reason));
        let updated = am.update(&txn).await?;

        txn.commit().await?;

        Ok(updated)
    }

    /// Withdraw a claim: delete if owned by the caller and still Pending.
    pub async fn withdraw(
        db: &DatabaseConnection,
        id: i32,
        owner_id: i32,
    ) -> Result<(), ClaimError> {
        let txn = db.begin().await?;

        let model = Self::find_model_by_id(&txn, id).await?;
        if model.submitted_by_user_id != owner_id {
            txn.rollback().await?;
            return Err(ClaimError::NotOwner);
        }
        if model.status != ClaimStatus::Pending {
            txn.rollback().await?;
            return Err(ClaimError::InvalidStatus);
        }

        claim_entity::Entity::delete_by_id(id).exec(&txn).await?;

        txn.commit().await?;

        Ok(())
    }
}
