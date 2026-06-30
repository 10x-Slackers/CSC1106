use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    Order, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::entity::party as party_entity;
use crate::entity::party::{PartyStatus, PartyType};
use crate::models::error::AppError;
use crate::models::util::{PER_PAGE, clamp_pagination, like_pattern};

use super::types::Party;

impl Party {
    async fn find_model_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<party_entity::Model>, AppError> {
        party_entity::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(AppError::from)
    }

    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<Party>, AppError> {
        Self::find_model_by_id(db, id)
            .await
            .map(|opt| opt.map(Party::from))
    }

    pub async fn find_by_email(
        db: &DatabaseConnection,
        email: &str,
    ) -> Result<Option<Party>, AppError> {
        party_entity::Entity::find()
            .filter(party_entity::Column::Email.eq(email))
            .one(db)
            .await
            .map_err(AppError::from)
            .map(|opt| opt.map(Party::from))
    }

    /// List parties with optional filters.
    /// `q` matches name or company (case-insensitive LIKE).
    pub async fn list(
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

    /// List all parties ordered by name.
    pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<Party>, AppError> {
        let parties = party_entity::Entity::find()
            .order_by(party_entity::Column::Name, Order::Asc)
            .all(db)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Party::from)
            .collect();
        Ok(parties)
    }

    /// List active parties ordered by name.
    pub async fn list_active(db: &DatabaseConnection) -> Result<Vec<Party>, AppError> {
        let parties = party_entity::Entity::find()
            .filter(party_entity::Column::Status.eq(PartyStatus::Active))
            .order_by(party_entity::Column::Name, Order::Asc)
            .all(db)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Party::from)
            .collect();
        Ok(parties)
    }

    /// Map of party id -> name for all parties.
    pub async fn name_map(
        db: &DatabaseConnection,
    ) -> Result<std::collections::HashMap<i32, String>, AppError> {
        let parties = party_entity::Entity::find().all(db).await?;
        Ok(parties.into_iter().map(|p| (p.id, p.name)).collect())
    }

    /// Look up a party name by ID. Returns `None` if the party does not exist.
    pub async fn find_name_by_id<C: ConnectionTrait>(
        db: &C,
        id: i32,
    ) -> Result<Option<String>, AppError> {
        party_entity::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(AppError::from)
            .map(|opt| opt.map(|p| p.name))
    }

    /// Build a map of party ID → name for the given IDs.
    pub async fn name_map_by_ids<C: ConnectionTrait>(
        db: &C,
        ids: Vec<i32>,
    ) -> Result<std::collections::HashMap<i32, String>, AppError> {
        if ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let parties = party_entity::Entity::find()
            .filter(party_entity::Column::Id.is_in(ids))
            .all(db)
            .await?;
        Ok(parties.into_iter().map(|p| (p.id, p.name)).collect())
    }

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

    /// Update party fields.
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

    /// Set party status (activate/deactivate).
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
