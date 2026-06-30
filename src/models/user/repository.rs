use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{
    Error as HashError, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    Order, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::entity::user as user_entity;
use crate::entity::user::{Role, UserStatus};
use crate::middleware::auth::UserCache;

use crate::models::error::{AppError, AuthError};
use crate::models::util::like_pattern;
use crate::routes::utils::{PER_PAGE, clamp_pagination};

use super::types::User;

impl User {
    async fn find_model_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<user_entity::Model>, AuthError> {
        user_entity::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(AuthError::from)
    }

    /// Verify a plaintext password against the stored Argon2 hash.
    pub fn verify_password(&self, password: &str) -> Result<(), HashError> {
        let parsed_hash = PasswordHash::new(&self.password_hash)?;
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash)
    }

    /// Create a new user with a hashed password.
    pub async fn create(
        db: &DatabaseConnection,
        email: &str,
        name: &str,
        password: &str,
        role: Role,
    ) -> Result<User, AuthError> {
        let password_hash = hash_password(password)?;

        let now = chrono::Utc::now().naive_utc();

        let user = user_entity::ActiveModel {
            email: Set(email.into()),
            name: Set(name.into()),
            password_hash: Set(password_hash),
            role: Set(role),
            disabled: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        user_entity::Entity::insert(user)
            .exec(db)
            .await
            .map_err(AuthError::from)?;

        Self::find_by_email(db, email)
            .await?
            .ok_or(AuthError::NotFound)
    }

    /// Look up a user by their primary key.
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<User>, AuthError> {
        Self::find_model_by_id(db, id)
            .await
            .map(|opt| opt.map(User::from))
    }

    /// Look up a user by email address.
    pub async fn find_by_email(
        db: &DatabaseConnection,
        email: &str,
    ) -> Result<Option<User>, AuthError> {
        user_entity::Entity::find()
            .filter(user_entity::Column::Email.eq(email))
            .one(db)
            .await
            .map_err(AuthError::from)
            .map(|opt| opt.map(User::from))
    }

    /// List users with optional filters.
    /// `q` matches name or email (case-insensitive LIKE).
    pub async fn list(
        db: &DatabaseConnection,
        q: Option<&str>,
        role: Option<Role>,
        status: Option<UserStatus>,
        page: u32,
    ) -> Result<(Vec<User>, u32, u32), AuthError> {
        let mut conditions = Condition::all();

        if let Some(query) = q
            && let Some(pattern) = like_pattern(query)
        {
            conditions = conditions.add(
                Condition::any()
                    .add(user_entity::Column::Name.like(&pattern))
                    .add(user_entity::Column::Email.like(&pattern)),
            );
        }

        if let Some(role) = role {
            conditions = conditions.add(user_entity::Column::Role.eq(role));
        }

        if let Some(status) = status {
            conditions = conditions.add(user_entity::Column::Disabled.eq(status.disabled()));
        }

        let paginator = user_entity::Entity::find()
            .filter(conditions)
            .order_by(user_entity::Column::CreatedAt, Order::Desc)
            .paginate(db, PER_PAGE);
        let num_pages = paginator.num_pages().await.map_err(AuthError::from)?;
        let (total_pages, page) = clamp_pagination(page, num_pages);
        let users = paginator
            .fetch_page((page - 1) as u64)
            .await
            .map_err(AuthError::from)?
            .into_iter()
            .map(User::from)
            .collect();

        Ok((users, total_pages, page))
    }

    /// Update user fields, re-hashing password if provided; invalidates cache.
    pub async fn update(
        &self,
        db: &DatabaseConnection,
        cache: &UserCache,
        email: Option<&str>,
        name: Option<&str>,
        password: Option<&str>,
        role: Option<Role>,
    ) -> Result<User, AuthError> {
        let user_model = Self::find_model_by_id(db, self.id)
            .await?
            .ok_or(AuthError::NotFound)?;

        let mut user: user_entity::ActiveModel = user_model.into();

        if let Some(email) = email {
            user.email = Set(email.into());
        }
        if let Some(name) = name {
            user.name = Set(name.into());
        }
        if let Some(password) = password {
            let password_hash = hash_password(password)?;
            user.password_hash = Set(password_hash);
        }
        if let Some(role) = role {
            user.role = Set(role);
        }

        user.updated_at = Set(chrono::Utc::now().naive_utc());

        let updated = user.update(db).await.map_err(AuthError::from)?;

        // Invalidate cache since user info changed
        cache.invalidate(&self.email);

        Ok(User::from(updated))
    }

    /// Enable or disable a user account; invalidates cache.
    pub async fn set_disabled(
        &self,
        db: &DatabaseConnection,
        cache: &UserCache,
        disabled: bool,
    ) -> Result<User, AuthError> {
        let user_model = Self::find_model_by_id(db, self.id)
            .await?
            .ok_or(AuthError::NotFound)?;

        let mut user: user_entity::ActiveModel = user_model.into();
        user.disabled = Set(disabled);
        user.updated_at = Set(chrono::Utc::now().naive_utc());

        let updated = user.update(db).await.map_err(AuthError::from)?;

        // Remove user from cache
        cache.invalidate(&self.email);

        Ok(User::from(updated))
    }

    /// Check if user exists, password is correct, and account is not disabled.
    pub async fn authenticate(
        db: &DatabaseConnection,
        email: &str,
        password: &str,
    ) -> Result<User, AuthError> {
        match Self::find_by_email(db, email).await? {
            Some(user) if !user.disabled && user.verify_password(password).is_ok() => Ok(user),
            _ => Err(AuthError::InvalidCredentials),
        }
    }
}

/// Look up a user's display name by ID. Returns `None` if the user does not exist.
pub async fn name_by_id<C: ConnectionTrait>(db: &C, id: i32) -> Result<Option<String>, AppError> {
    let user = user_entity::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AppError::from)?;
    Ok(user.map(|u| u.name))
}

/// Hash a plaintext password using Argon2 with a random salt.
fn hash_password(password: &str) -> Result<String, HashError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}
