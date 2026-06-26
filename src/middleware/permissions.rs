use actix_web::dev::{Payload, ServiceResponse};
use actix_web::middleware::ErrorHandlerResponse;
use actix_web::{FromRequest, HttpRequest, HttpResponse, web};
use std::future::Future;
use std::pin::Pin;
use tera::{Context, Tera};

use crate::middleware::auth::Authenticated;

pub trait RoleSet {
    const ROLES: &'static [crate::entity::user::Role];
}

pub struct AdminOnly;
impl RoleSet for AdminOnly {
    const ROLES: &'static [crate::entity::user::Role] = &[crate::entity::user::Role::Admin];
}

pub struct Finance;
impl RoleSet for Finance {
    const ROLES: &'static [crate::entity::user::Role] = &[
        crate::entity::user::Role::Admin,
        crate::entity::user::Role::Accountant,
    ];
}

/// Look up a named role set by its short name.
pub fn role_set(name: &str) -> Option<&'static [crate::entity::user::Role]> {
    match name.trim().to_lowercase().as_str() {
        "admin" => Some(AdminOnly::ROLES),
        "finance" => Some(Finance::ROLES),
        _ => None,
    }
}

/// Extractor that requires the user's role to be in `R::ROLES`.
pub struct Require<R: RoleSet> {
    pub user: Authenticated,
    // Zero-sized marker so the generic `R` is used.
    _marker: std::marker::PhantomData<R>,
}

/// Lets handlers use `user.id`, `user.role`, etc. directly via auto-deref, making `Require<R>` a drop-in replacement for `Authenticated`.
impl<R: RoleSet> std::ops::Deref for Require<R> {
    type Target = Authenticated;
    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

impl<R: RoleSet> FromRequest for Require<R> {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        // Reuse Authenticated so auth logic stays in one place.
        let auth_fut = <Authenticated as FromRequest>::from_request(req, payload);

        Box::pin(async move {
            let authenticated = auth_fut.await?;

            if !R::ROLES.contains(&authenticated.role) {
                return Err(actix_web::error::ErrorForbidden("Insufficient permissions"));
            }

            Ok(Require {
                user: authenticated,
                _marker: std::marker::PhantomData,
            })
        })
    }
}

/// Render a forbidden page for insufficient permissions.
pub fn forbidden_page<B>(
    res: ServiceResponse<B>,
) -> Result<ErrorHandlerResponse<B>, actix_web::Error> {
    let (req, _) = res.into_parts();
    let tera = req
        .app_data::<web::Data<Tera>>()
        .expect("Tera not registered");
    let context = Context::new();

    let rendered = tera.render("auth/forbidden.html", &context).unwrap_or_else(|e| {
        eprintln!("Template error: {e}");
        String::from(
            "<html><body><h1>Forbidden</h1><p>You don't have permission to access this page.</p><a href=\"/\">Go home</a></body></html>",
        )
    });

    let new_response = HttpResponse::Forbidden()
        .content_type("text/html")
        .body(rendered);
    let response = new_response.map_into_right_body();
    Ok(ErrorHandlerResponse::Response(ServiceResponse::new(
        req, response,
    )))
}
