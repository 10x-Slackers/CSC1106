//! Authentication middleware for Actix-web.
//!
//! Authors: Kitsuneez

use actix_identity::Identity;
use actix_web::dev::{Payload, ServiceResponse};
use actix_web::error::ErrorUnauthorized;
use actix_web::middleware::ErrorHandlerResponse;
use actix_web::{FromRequest, HttpRequest, HttpResponse, web};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tera::{Context, Tera};

use crate::entity::user::Role;
use crate::models::user::User;

const CACHE_TTL: Duration = Duration::from_secs(300);

/// Cache Manager to store authenticated users
#[derive(Clone)]
pub struct UserCache {
    inner: web::Data<RwLock<HashMap<String, CachedUser>>>,
}

pub struct CachedUser {
    pub user: User,
    pub cached_at: Instant,
}

impl UserCache {
    /// initialise the cache
    pub fn new() -> Self {
        Self {
            inner: web::Data::new(RwLock::new(HashMap::new())),
        }
    }

    /// insert/update user into cache with current timestamp and email as key
    pub fn insert(&self, user: &User) {
        self.inner.write().unwrap().insert(
            user.email.clone(),
            CachedUser {
                user: user.clone(),
                cached_at: Instant::now(),
            },
        );
    }

    /// remove user from cache by email
    pub fn invalidate(&self, email: &str) {
        self.inner.write().unwrap().remove(email);
    }

    /// checks if user is in cache and kicks user if expired
    fn get(&self, email: &str) -> Option<CachedUser> {
        let cache = self.inner.read().unwrap();
        let entry = cache.get(email)?;
        if entry.cached_at.elapsed() < CACHE_TTL {
            Some(CachedUser {
                user: entry.user.clone(),
                cached_at: entry.cached_at,
            })
        } else {
            drop(cache);
            self.inner.write().unwrap().remove(email);
            None
        }
    }
}

#[derive(serde::Serialize)]
pub struct Authenticated {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub role: Role,
}

impl FromRequest for Authenticated {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    /// extracts user from request, checks cache first, then database if not found, and handles disabled accounts
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

            let user = User::find_by_email(db.get_ref(), &email)
                .await
                .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
                .ok_or_else(|| ErrorUnauthorized("User not found"))?;

            if user.disabled {
                return Err(ErrorUnauthorized("Account disabled"));
            }

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

/// Redirects unauthorized users to the login page
pub fn redirect_unauthorized<B>(
    res: ServiceResponse<B>,
) -> Result<ErrorHandlerResponse<B>, actix_web::Error> {
    let (req, _) = res.into_parts();
    let new_response = HttpResponse::Found()
        .append_header(("Location", "/login"))
        .finish();
    let response = new_response.map_into_right_body();
    Ok(ErrorHandlerResponse::Response(ServiceResponse::new(
        req, response,
    )))
}

/// Renders an unauthorized page for users without permission
pub fn show_unauthorized_page<B>(
    res: ServiceResponse<B>,
) -> Result<ErrorHandlerResponse<B>, actix_web::Error> {
    let (req, _) = res.into_parts();
    let tera = req
        .app_data::<web::Data<Tera>>()
        .expect("Tera not registered");
    let context = Context::new();
    let rendered = tera.render("auth/unauthorized.html", &context).unwrap_or_else(|e| {
        eprintln!("Template error: {e}");
        String::from(
            "<html><body><h1>Unauthorized</h1><a href=\"/login\">Proceed to login</a></body></html>",
        )
    });
    let new_response = HttpResponse::Unauthorized()
        .content_type("text/html")
        .body(rendered);
    let response = new_response.map_into_right_body();
    Ok(ErrorHandlerResponse::Response(ServiceResponse::new(
        req, response,
    )))
}
