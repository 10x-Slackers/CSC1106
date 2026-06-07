use actix_identity::Identity;
use actix_web::dev::Payload;
use actix_web::error::ErrorUnauthorized;
use actix_web::{FromRequest, HttpRequest};
use sea_orm::DatabaseConnection;
use std::future::Future;
use std::pin::Pin;

use crate::entity::role::Role;
use crate::models::user::User;

/// Extractor that requires authentication and looks up the user.
/// Returns 401 for unauthenticated requests.
#[allow(dead_code)] // TODO: ACL not implemented yet
pub struct Authenticated {
    pub email: String,
    pub role: Role,
}

impl FromRequest for Authenticated {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let result = <Identity as FromRequest>::from_request(req, payload);
        let db = req
            .app_data::<actix_web::web::Data<DatabaseConnection>>()
            .expect("DatabaseConnection not in app data")
            .clone();

        Box::pin(async move {
            let identity = result.await?;
            let email = identity
                .id()
                .map_err(|_| ErrorUnauthorized("Invalid session"))?;

            let user = User::find_by_email(db.get_ref(), &email)
                .await
                .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?
                .ok_or_else(|| ErrorUnauthorized("User not found"))?;

            Ok(Authenticated {
                email: user.email.clone(),
                role: user.role,
            })
        })
    }
}
