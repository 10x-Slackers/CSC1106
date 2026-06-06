use actix_identity::Identity;
use actix_web::dev::Payload;
use actix_web::error::ErrorUnauthorized;
use actix_web::{FromRequest, HttpRequest};
use std::future::Future;
use std::pin::Pin;

use crate::models::user::{Role, find_user};

/// Extractor that requires authentication and looks up the user.
/// Returns 401 for unauthenticated requests.
#[allow(dead_code)]
pub struct Authenticated {
    pub email: String,
    pub role: Role,
}

impl FromRequest for Authenticated {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let result = <Identity as FromRequest>::from_request(req, payload);

        Box::pin(async move {
            let identity = result.await?;
            let email = identity
                .id()
                .map_err(|_| ErrorUnauthorized("Invalid session"))?;
            let user = find_user(&email).ok_or_else(|| ErrorUnauthorized("User not found"))?;

            Ok(Authenticated {
                email: user.email.clone(),
                role: user.role.clone(),
            })
        })
    }
}
