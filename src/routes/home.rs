use actix_web::http::StatusCode;
use actix_web::middleware::ErrorHandlers;
use actix_web::{Responder, web};
use tera::Context;

use crate::middleware::auth::{Authenticated, redirect_unauthorized};
use crate::routes::utils::insert_nav_context;
use crate::routes::utils::render;

/// Render the home page for authenticated users.
pub async fn home(user: Authenticated, tera: web::Data<tera::Tera>) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);
    render(&tera, "home/index.html", &context)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/")
            .wrap(ErrorHandlers::new().handler(StatusCode::UNAUTHORIZED, redirect_unauthorized))
            .route(web::get().to(home)),
    );
}
