//! Contains the main database and authentication logic for users.
//! Here is where user accounts are created, searched, updated, disabled, and authenticated.
//!
//! Authors: Tan Yong Meng

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{
    Error as HashError, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::entity::user as user_entity;
use crate::entity::user::{Role, UserStatus};
use crate::models::error::{AppError, AuthError};
use crate::models::util::{PER_PAGE, clamp_pagination, like_pattern};

use super::types::User;

impl User {
    /// Finds the raw SeaORM user model by its ID.
    async fn find_model_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<user_entity::Model>, AuthError> {
        user_entity::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(AuthError::from)
    }

    /// Verifies a plaintext password against the user's stored password hash.
    ///
    /// Returns `Ok(())` if the password matches the hash.
    pub fn verify_password(&self, password: &str) -> Result<(), HashError> {
        let parsed_hash = PasswordHash::new(&self.password_hash)?;
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash)
    }

    /// Creates a new user account.
    ///
    /// The provided password is hashed before being stored. New users are
    /// created as enabled by default.
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

    /// Finds a user by their ID.
    ///
    /// Returns `Ok(None)` if no matching user exists.
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<User>, AuthError> {
        Self::find_model_by_id(db, id)
            .await
            .map(|opt| opt.map(User::from))
    }

    /// Finds a user by their email address.
    ///
    /// Returns `Ok(None)` if no matching user exists.
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

    /// Searches for users using optional filters and pagination.
    ///
    /// The search can filter by name/email query, role, and account status.
    /// Results are ordered by creation date in descending order.
    ///
    /// Returns a tuple containing:
    /// - the users for the selected page,
    /// - the total number of pages,
    /// - the clamped current page number.
    pub async fn search(
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

    /// Updates an existing user account.
    ///
    /// Each field is optional. Fields set to `None` are left unchanged.
    /// If a new password is provided, it is hashed before being saved.
    pub async fn update(
        &self,
        db: &DatabaseConnection,
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

        Ok(User::from(updated))
    }

    /// Enables or disables a user account.
    ///
    /// When `disabled` is `true`, the user account should no longer be allowed
    /// to authenticate.
    pub async fn set_disabled(
        &self,
        db: &DatabaseConnection,
        disabled: bool,
    ) -> Result<User, AuthError> {
        let user_model = Self::find_model_by_id(db, self.id)
            .await?
            .ok_or(AuthError::NotFound)?;

        let mut user: user_entity::ActiveModel = user_model.into();
        user.disabled = Set(disabled);
        user.updated_at = Set(chrono::Utc::now().naive_utc());

        let updated = user.update(db).await.map_err(AuthError::from)?;

        Ok(User::from(updated))
    }

    /// Authenticates a user using an email address and password.
    ///
    /// Authentication succeeds only when the user exists, the account is not
    /// disabled, and the provided password matches the stored password hash.
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

/// Finds a user's name by their ID.
///
/// Returns `Ok(None)` if no matching user exists.
pub async fn name_by_id<C: ConnectionTrait>(db: &C, id: i32) -> Result<Option<String>, AppError> {
    let name: Option<String> = user_entity::Entity::find_by_id(id)
        .select_only()
        .column(user_entity::Column::Name)
        .into_tuple()
        .one(db)
        .await?;
    Ok(name)
}

/// Builds a map of user IDs to user names.
///
/// Returns an empty map when the provided ID list is empty.
pub async fn name_map_by_ids<C: ConnectionTrait>(
    db: &C,
    ids: Vec<i32>,
) -> Result<std::collections::HashMap<i32, String>, AppError> {
    if ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let rows: Vec<(i32, String)> = user_entity::Entity::find()
        .select_only()
        .column(user_entity::Column::Id)
        .column(user_entity::Column::Name)
        .filter(user_entity::Column::Id.is_in(ids))
        .into_tuple()
        .all(db)
        .await?;
    Ok(rows.into_iter().collect())
}

/// Hashes a plaintext password using Argon2.
///
/// A random salt is generated for each password before hashing. The returned
/// string contains the encoded password hash and parameters needed for later
/// verification.
fn hash_password(password: &str) -> Result<String, HashError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}
