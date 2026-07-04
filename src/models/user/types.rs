use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::entity::user as user_entity;
use crate::entity::user::Role;

/// User Model to define a User Record for Application-Level Use.
#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: Role,
    pub disabled: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<user_entity::Model> for User {
    /// Converts a SeaORM user entity model into an application [`User`].
    fn from(m: user_entity::Model) -> Self {
        User {
            id: m.id,
            name: m.name,
            email: m.email,
            password_hash: m.password_hash,
            role: m.role,
            disabled: m.disabled,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
