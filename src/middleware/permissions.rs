use actix_web::dev::{Payload, ServiceResponse};
use actix_web::middleware::ErrorHandlerResponse;
use actix_web::{FromRequest, HttpRequest, HttpResponse, web};
use std::future::Future;
use std::pin::Pin;
use tera::{Context, Tera};

use crate::entity::user::Role;
use crate::middleware::auth::Authenticated;

pub trait RoleSet {
    const ROLES: &'static [Role];
}

pub struct AdminOnly;
impl RoleSet for AdminOnly {
    const ROLES: &'static [Role] = &[Role::Admin];
}

pub struct Finance;
impl RoleSet for Finance {
    const ROLES: &'static [Role] = &[Role::Admin, Role::Accountant];
}

/// returns the role set based on the provided name, or None if not found
pub fn role_set(name: &str) -> Option<&'static [Role]> {
    let name = name.trim();
    if name.eq_ignore_ascii_case("admin") {
        Some(AdminOnly::ROLES)
    } else if name.eq_ignore_ascii_case("finance") {
        Some(Finance::ROLES)
    } else {
        None
    }
}

/// wraps authenticated user together with the required role for a route
pub struct Require<R: RoleSet> {
    pub user: Authenticated,
    _marker: std::marker::PhantomData<R>,
}

/// allow route code to be written easier such as user.id instead of user.user.id
impl<R: RoleSet> std::ops::Deref for Require<R> {
    type Target = Authenticated;
    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

/// checks whether the user has the required role(s) and returns 403 Forbidden if not
impl<R: RoleSet> FromRequest for Require<R> {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
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

/// returns a 403 Forbidden page for users without permission
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
