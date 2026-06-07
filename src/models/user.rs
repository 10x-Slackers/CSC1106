use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{Error as HashError, SaltString};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use crate::entity::role::Role;
use crate::entity::user as user_entity;

#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub role: Role,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum AuthError {
    InvalidCredentials,
    DatabaseError(String),
}

impl From<user_entity::Model> for User {
    fn from(m: user_entity::Model) -> Self {
        User {
            id: m.id,
            name: m.name,
            email: m.email,
            password_hash: m.password_hash,
            role: m.role,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

fn hash_password(password: &str) -> Result<String, HashError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

fn verify_password(password: &str, password_hash: &str) -> Result<(), HashError> {
    let parsed_hash = PasswordHash::new(password_hash)?;
    Argon2::default().verify_password(password.as_bytes(), &parsed_hash)
}

pub async fn create_user(
    db: &DatabaseConnection,
    email: &str,
    name: &str,
    password: &str,
    role: Role,
) -> Result<User, AuthError> {
    let password_hash =
        hash_password(password).map_err(|e| AuthError::DatabaseError(e.to_string()))?;

    let now = chrono::Utc::now().naive_utc();

    let user = user_entity::ActiveModel {
        email: Set(email.into()),
        name: Set(name.into()),
        password_hash: Set(password_hash),
        role: Set(role),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    user_entity::Entity::insert(user)
        .exec(db)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

    find_user_by_email(db, email)
        .await?
        .ok_or_else(|| AuthError::DatabaseError("User not found after insert".into()))
}

#[allow(dead_code)] // TODO: Remove when implementing user management
async fn find_user_model_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<user_entity::Model>, AuthError> {
    user_entity::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))
}

#[allow(dead_code)] // TODO: Remove when implementing user management
pub async fn find_user_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<User>, AuthError> {
    find_user_model_by_id(db, id)
        .await
        .map(|opt| opt.map(User::from))
}

pub async fn find_user_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<User>, AuthError> {
    user_entity::Entity::find()
        .filter(user_entity::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))
        .map(|opt| opt.map(User::from))
}

#[allow(dead_code)] // TODO: Remove when implementing user management
pub async fn update_user(
    db: &DatabaseConnection,
    user_id: i32,
    email: Option<&str>,
    name: Option<&str>,
    password: Option<&str>,
    role: Option<Role>,
) -> Result<User, AuthError> {
    let user_model = find_user_model_by_id(db, user_id)
        .await?
        .ok_or_else(|| AuthError::DatabaseError("User not found".into()))?;

    let mut user: user_entity::ActiveModel = user_model.into();

    if let Some(email) = email {
        user.email = Set(email.into());
    }
    if let Some(name) = name {
        user.name = Set(name.into());
    }
    if let Some(password) = password {
        let password_hash =
            hash_password(password).map_err(|e| AuthError::DatabaseError(e.to_string()))?;
        user.password_hash = Set(password_hash);
    }
    if let Some(role) = role {
        user.role = Set(role);
    }

    user.updated_at = Set(chrono::Utc::now().naive_utc());

    let updated = user
        .update(db)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

    Ok(User::from(updated))
}

pub async fn authenticate(
    db: &DatabaseConnection,
    email: &str,
    password: &str,
) -> Result<User, AuthError> {
    match find_user_by_email(db, email).await? {
        Some(user) if verify_password(password, &user.password_hash).is_ok() => Ok(user),
        _ => Err(AuthError::InvalidCredentials),
    }
}
