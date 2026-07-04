//! Contains the main database logic for parties.
//!
//! Authors: Tan Yong Meng

use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::entity::party as party_entity;
use crate::entity::party::{PartyStatus, PartyType};
use crate::models::error::AppError;
use crate::models::util::{PER_PAGE, clamp_pagination, like_pattern};

use super::types::Party;

impl Party {
    /// Finds the raw SeaORM party model by ID.
    ///
    /// An internal helper used by other party lookup methods.
    async fn find_model_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<party_entity::Model>, AppError> {
        party_entity::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(AppError::from)
    }

    /// Finds a party by its ID
    ///
    /// Returns `Ok(None)` if no matching party exists.
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<Party>, AppError> {
        Self::find_model_by_id(db, id)
            .await
            .map(|opt| opt.map(Party::from))
    }

    /// Searches for parties using optional filters and pagination.
    ///
    /// The search can filter parties by:
    /// - name or company search query,
    /// - party type,
    /// - party status.
    ///
    /// Results are ordered by creation date in descending order.
    ///
    /// Returns a tuple containing:
    /// - `Vec<Party>`: the parties for the selected page,
    /// - `u32`: the total number of pages,
    /// - `u32`: the current page number after pagination clamping.
    pub async fn search(
        db: &DatabaseConnection,
        q: Option<&str>,
        party_type: Option<PartyType>,
        status: Option<PartyStatus>,
        page: u32,
    ) -> Result<(Vec<Party>, u32, u32), AppError> {
        let mut conditions = Condition::all();

        if let Some(query) = q
            && let Some(pattern) = like_pattern(query)
        {
            conditions = conditions.add(
                Condition::any()
                    .add(party_entity::Column::Name.like(&pattern))
                    .add(party_entity::Column::Company.like(&pattern)),
            );
        }

        if let Some(pt) = party_type {
            conditions = conditions.add(party_entity::Column::PartyType.eq(pt));
        }

        if let Some(s) = status {
            conditions = conditions.add(party_entity::Column::Status.eq(s));
        }

        let paginator = party_entity::Entity::find()
            .filter(conditions)
            .order_by(party_entity::Column::CreatedAt, Order::Desc)
            .paginate(db, PER_PAGE);
        let num_pages = paginator.num_pages().await.map_err(AppError::from)?;
        let (total_pages, page) = clamp_pagination(page, num_pages);
        let parties = paginator
            .fetch_page((page - 1) as u64)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Party::from)
            .collect();

        Ok((parties, total_pages, page))
    }

    /// Lists parties using optional party type and status filters.
    ///
    /// Results are ordered alphabetically by party name.
    pub async fn list(
        db: &DatabaseConnection,
        party_type: Option<PartyType>,
        status: Option<PartyStatus>,
    ) -> Result<Vec<Party>, AppError> {
        let mut query = party_entity::Entity::find();
        if let Some(pt) = party_type {
            query = query.filter(party_entity::Column::PartyType.eq(pt));
        }
        if let Some(s) = status {
            query = query.filter(party_entity::Column::Status.eq(s));
        }
        let parties = query
            .order_by(party_entity::Column::Name, Order::Asc)
            .all(db)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Party::from)
            .collect();
        Ok(parties)
    }

    /// Builds a map of party IDs to party names.
    ///
    /// Returns an empty map when the provided ID list is empty.
    pub async fn name_map_by_ids<C: ConnectionTrait>(
        db: &C,
        ids: Vec<i32>,
    ) -> Result<std::collections::HashMap<i32, String>, AppError> {
        if ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(i32, String)> = party_entity::Entity::find()
            .select_only()
            .column(party_entity::Column::Id)
            .column(party_entity::Column::Name)
            .filter(party_entity::Column::Id.is_in(ids))
            .into_tuple()
            .all(db)
            .await?;
        Ok(rows.into_iter().collect())
    }

    /// Creates a new party record.
    ///
    /// New parties are created with [`PartyStatus::Active`] by default.
    pub async fn create(
        db: &DatabaseConnection,
        party_type: PartyType,
        name: &str,
        company: Option<&str>,
        email: &str,
        phone: &str,
        address: &str,
    ) -> Result<Party, AppError> {
        let now = chrono::Utc::now().naive_utc();

        let party = party_entity::ActiveModel {
            party_type: Set(party_type),
            name: Set(name.into()),
            company: Set(company.map(|s| s.into())),
            email: Set(email.trim().to_lowercase()),
            phone: Set(phone.into()),
            address: Set(address.into()),
            status: Set(PartyStatus::Active),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let result = party_entity::Entity::insert(party)
            .exec(db)
            .await
            .map_err(AppError::from)?;

        Self::find_model_by_id(db, result.last_insert_id)
            .await?
            .map(Party::from)
            .ok_or(AppError::NotFound)
    }

    /// Updates an existing party record.
    ///
    /// Each field is optional. Fields set to `None` are left unchanged.
    ///
    /// The `updated_at` timestamp is refreshed before saving.
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        db: &DatabaseConnection,
        party_type: Option<PartyType>,
        name: Option<&str>,
        company: Option<Option<&str>>,
        email: Option<&str>,
        phone: Option<&str>,
        address: Option<&str>,
    ) -> Result<Party, AppError> {
        let mut party: party_entity::ActiveModel = party_entity::ActiveModel {
            id: Set(self.id),
            party_type: Set(self.party_type.clone()),
            name: Set(self.name.clone()),
            company: Set(self.company.clone()),
            email: Set(self.email.clone()),
            phone: Set(self.phone.clone()),
            address: Set(self.address.clone()),
            status: Set(self.status.clone()),
            created_at: Set(self.created_at),
            updated_at: Set(self.updated_at),
        };

        if let Some(pt) = party_type {
            party.party_type = Set(pt);
        }
        if let Some(name) = name {
            party.name = Set(name.into());
        }
        if let Some(company) = company {
            party.company = Set(company.map(|s| s.into()));
        }
        if let Some(email) = email {
            party.email = Set(email.trim().to_lowercase());
        }
        if let Some(phone) = phone {
            party.phone = Set(phone.into());
        }
        if let Some(address) = address {
            party.address = Set(address.into());
        }

        party.updated_at = Set(chrono::Utc::now().naive_utc());

        let updated = party.update(db).await.map_err(AppError::from)?;

        Ok(Party::from(updated))
    }

    /// Updates the status of an existing party.
    ///
    /// This is used to activate or deactivate a party without changing the
    /// rest of the party details.
    pub async fn set_status(
        &self,
        db: &DatabaseConnection,
        status: PartyStatus,
    ) -> Result<Party, AppError> {
        let mut party: party_entity::ActiveModel = party_entity::ActiveModel {
            id: Set(self.id),
            party_type: Set(self.party_type.clone()),
            name: Set(self.name.clone()),
            company: Set(self.company.clone()),
            email: Set(self.email.clone()),
            phone: Set(self.phone.clone()),
            address: Set(self.address.clone()),
            status: Set(self.status.clone()),
            created_at: Set(self.created_at),
            updated_at: Set(self.updated_at),
        };
        party.status = Set(status);
        party.updated_at = Set(chrono::Utc::now().naive_utc());

        let updated = party.update(db).await.map_err(AppError::from)?;

        Ok(Party::from(updated))
    }
}
