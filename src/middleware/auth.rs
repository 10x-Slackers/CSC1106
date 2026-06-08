use actix_identity::Identity;
use actix_web::dev::Payload;
use actix_web::error::ErrorUnauthorized;
use actix_web::{FromRequest, HttpRequest, web};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::models::user::User;

const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

/// In-memory user cache keyed by email
#[derive(Clone)]
pub struct UserCache {
    inner: web::Data<RwLock<HashMap<String, CachedUser>>>,
}

pub struct CachedUser {
    pub user: User,
    pub cached_at: Instant,
}

impl UserCache {
    pub fn new() -> Self {
        Self {
            inner: web::Data::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn insert(&self, user: &User) {
        self.inner.write().unwrap().insert(
            user.email.clone(),
            CachedUser {
                user: user.clone(),
                cached_at: Instant::now(),
            },
        );
    }

    pub fn invalidate(&self, email: &str) {
        self.inner.write().unwrap().remove(email);
    }

    fn get(&self, email: &str) -> Option<CachedUser> {
        let cache = self.inner.read().unwrap();
        let entry = cache.get(email)?;
        if entry.cached_at.elapsed() < CACHE_TTL {
            Some(CachedUser {
                user: entry.user.clone(),
                cached_at: entry.cached_at,
            })
        } else {
            // Drop read lock before acquiring write lock to remove expired entry
            drop(cache);
            self.inner.write().unwrap().remove(email);
            None
        }
    }
}

/// Extractor that requires authentication and looks up the user.
/// Returns 401 for unauthenticated or disabled users.
#[allow(dead_code)] // TODO: ACL not implemented yet
pub struct Authenticated {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub role: crate::entity::user::Role,
}

impl FromRequest for Authenticated {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let result = <Identity as FromRequest>::from_request(req, payload);
        let db = req
            .app_data::<web::Data<DatabaseConnection>>()
            .expect("DatabaseConnection not in app data")
            .clone();
        let cache = req
            .app_data::<web::Data<UserCache>>()
            .expect("UserCache not in app data")
            .clone();

        Box::pin(async move {
            let identity = result.await?;
            let email = identity
                .id()
                .map_err(|_| ErrorUnauthorized("Invalid session"))?;

            // Check cache first
            if let Some(entry) = cache.get(&email) {
                if entry.user.disabled {
                    return Err(ErrorUnauthorized("Account disabled"));
                }
                return Ok(Authenticated {
                    id: entry.user.id,
                    name: entry.user.name.clone(),
                    email: entry.user.email.clone(),
                    role: entry.user.role.clone(),
                });
            }

            // If not cached, lookup database
            let user = User::find_by_email(db.get_ref(), &email)
                .await
                .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
                .ok_or_else(|| ErrorUnauthorized("User not found"))?;

            if user.disabled {
                return Err(ErrorUnauthorized("Account disabled"));
            }

            // Repopulate cache for non-cached users
            cache.insert(&user);

            Ok(Authenticated {
                id: user.id,
                name: user.name.clone(),
                email: user.email.clone(),
                role: user.role.clone(),
            })
        })
    }
}
