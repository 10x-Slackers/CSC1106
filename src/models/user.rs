use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
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

pub async fn create_user(
    db: &DatabaseConnection,
    email: &str,
    name: &str,
    password_hash: &str,
    role: Role,
) -> Result<User, AuthError> {
    let now = chrono::Utc::now().naive_utc();

    let user = user_entity::ActiveModel {
        email: Set(email.into()),
        name: Set(name.into()),
        password_hash: Set(password_hash.into()),
        role: Set(role),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    user_entity::Entity::insert(user)
        .exec(db)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

    find_user(db, email)
        .await?
        .ok_or_else(|| AuthError::DatabaseError("User not found after insert".into()))
}

pub async fn find_user(db: &DatabaseConnection, email: &str) -> Result<Option<User>, AuthError> {
    user_entity::Entity::find()
        .filter(user_entity::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))
        .map(|opt| opt.map(User::from))
}

pub async fn authenticate(
    db: &DatabaseConnection,
    email: &str,
    password: &str,
) -> Result<User, AuthError> {
    match find_user(db, email).await? {
        // TODO: Replace with proper password verification using argon2
        Some(user) if user.password_hash == password => Ok(user),
        _ => Err(AuthError::InvalidCredentials),
    }
}
