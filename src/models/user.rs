use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Clone, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Accountant,
    Staff,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub role: Role,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub enum AuthError {
    InvalidCredentials,
}

static USERS: LazyLock<Vec<User>> = LazyLock::new(|| {
    use chrono::NaiveDate;
    let ts = NaiveDate::from_ymd_opt(2025, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    vec![
        User {
            user_id: 1,
            name: "Administrator".into(),
            email: "admin@example.com".into(),
            password_hash: "P@ssw0rd".into(),
            role: Role::Admin,
            created_at: ts,
            updated_at: ts,
        },
        User {
            user_id: 2,
            name: "John Doe".into(),
            email: "john@example.com".into(),
            password_hash: "P@ssw0rd".into(),
            role: Role::Accountant,
            created_at: ts,
            updated_at: ts,
        },
    ]
});

// TODO: Replace this with a real database lookup
pub fn find_user(email: &str) -> Option<&'static User> {
    USERS.iter().find(|u| u.email == email)
}

// TODO: Replace with proper hashed password verification (argon2)
pub fn authenticate(email: &str, password: &str) -> Result<&'static User, AuthError> {
    match find_user(email) {
        Some(user) if user.password_hash == password => Ok(user),
        _ => Err(AuthError::InvalidCredentials),
    }
}
