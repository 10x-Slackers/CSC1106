use actix_web::http::StatusCode;
use actix_web::middleware::ErrorHandlers;
use actix_web::{HttpResponse, Responder, web};
use tera::{Context, Tera};

use crate::middleware::auth::{Authenticated, redirect_unauthorized};
use crate::routes::nav::insert_nav_context;

/// Render the home page for authenticated users.
pub async fn home(user: Authenticated, tera: web::Data<Tera>) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);

    match tera.render("home.html", &context) {
        Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
        Err(e) => {
            eprintln!("Template error: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/")
            .wrap(ErrorHandlers::new().handler(StatusCode::UNAUTHORIZED, redirect_unauthorized))
            .route(web::get().to(home)),
    );
}
