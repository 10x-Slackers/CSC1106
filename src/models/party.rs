use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, Order, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};

use crate::entity::party as party_entity;
use crate::entity::party::{PartyStatus, PartyType};
use crate::models::user::AuthError;

impl PartyType {
    /// Parse a party type string into a `PartyType`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "customer" => Some(PartyType::Customer),
            "vendor" => Some(PartyType::Vendor),
            _ => None,
        }
    }
}

impl PartyStatus {
    /// Parse a party status string into a `PartyStatus`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "active" => Some(PartyStatus::Active),
            "inactive" => Some(PartyStatus::Inactive),
            _ => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: i32,
    pub party_type: PartyType,
    pub name: String,
    pub company: Option<String>,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub status: PartyStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<party_entity::Model> for Party {
    fn from(m: party_entity::Model) -> Self {
        Party {
            id: m.id,
            party_type: m.party_type,
            name: m.name,
            company: m.company,
            email: m.email,
            phone: m.phone,
            address: m.address,
            status: m.status,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

impl Party {
    async fn find_model_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<party_entity::Model>, AuthError> {
        party_entity::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(AuthError::from)
    }

    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<Party>, AuthError> {
        Self::find_model_by_id(db, id)
            .await
            .map(|opt| opt.map(Party::from))
    }

    pub async fn find_by_email(
        db: &DatabaseConnection,
        email: &str,
    ) -> Result<Option<Party>, AuthError> {
        party_entity::Entity::find()
            .filter(party_entity::Column::Email.eq(email))
            .one(db)
            .await
            .map_err(AuthError::from)
            .map(|opt| opt.map(Party::from))
    }

    /// List parties with optional filters.
    /// `q` matches name or company (case-insensitive LIKE).
    pub async fn list(
        db: &DatabaseConnection,
        q: Option<&str>,
        party_type: Option<PartyType>,
        status: Option<PartyStatus>,
    ) -> Result<Vec<Party>, AuthError> {
        let mut conditions = Condition::all();

        if let Some(query) = q {
            let query = query.trim();
            if !query.is_empty() {
                let escaped = query.replace('%', "\\%").replace('_', "\\_");
                let pattern = format!("%{}%", escaped);
                conditions = conditions.add(
                    Condition::any()
                        .add(party_entity::Column::Name.like(&pattern))
                        .add(party_entity::Column::Company.like(&pattern)),
                );
            }
        }

        if let Some(pt) = party_type {
            conditions = conditions.add(party_entity::Column::PartyType.eq(pt));
        }

        if let Some(s) = status {
            conditions = conditions.add(party_entity::Column::Status.eq(s));
        }

        let parties = party_entity::Entity::find()
            .filter(conditions)
            .order_by(party_entity::Column::CreatedAt, Order::Desc)
            .all(db)
            .await
            .map_err(AuthError::from)?
            .into_iter()
            .map(Party::from)
            .collect();

        Ok(parties)
    }

    pub async fn create(
        db: &DatabaseConnection,
        party_type: PartyType,
        name: &str,
        company: Option<&str>,
        email: &str,
        phone: &str,
        address: &str,
    ) -> Result<Party, AuthError> {
        let now = chrono::Utc::now().naive_utc();

        let party = party_entity::ActiveModel {
            party_type: Set(party_type),
            name: Set(name.into()),
            company: Set(company.map(|s| s.into())),
            email: Set(email.into()),
            phone: Set(phone.into()),
            address: Set(address.into()),
            status: Set(PartyStatus::Active),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        party_entity::Entity::insert(party)
            .exec(db)
            .await
            .map_err(AuthError::from)?;

        Self::find_by_email(db, email)
            .await?
            .ok_or(AuthError::NotFound)
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
    ) -> Result<Party, AuthError> {
        let party_model = Self::find_model_by_id(db, self.id)
            .await?
            .ok_or(AuthError::NotFound)?;

        let mut party: party_entity::ActiveModel = party_model.into();

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
            party.email = Set(email.into());
        }
        if let Some(phone) = phone {
            party.phone = Set(phone.into());
        }
        if let Some(address) = address {
            party.address = Set(address.into());
        }

        party.updated_at = Set(chrono::Utc::now().naive_utc());

        let updated = party.update(db).await.map_err(AuthError::from)?;

        Ok(Party::from(updated))
    }

    /// Set party status (activate/deactivate).
    pub async fn set_status(
        &self,
        db: &DatabaseConnection,
        status: PartyStatus,
    ) -> Result<Party, AuthError> {
        let party_model = Self::find_model_by_id(db, self.id)
            .await?
            .ok_or(AuthError::NotFound)?;

        let mut party: party_entity::ActiveModel = party_model.into();
        party.status = Set(status);
        party.updated_at = Set(chrono::Utc::now().naive_utc());

        let updated = party.update(db).await.map_err(AuthError::from)?;

        Ok(Party::from(updated))
    }
}
