use actix_web::{HttpResponse, Responder, get, web};
use tera::{Context, Tera};

use crate::middleware::auth::Authenticated;
use crate::routes::nav::insert_nav_context;

#[get("/claims")]
pub async fn claims(user: Authenticated, tera: web::Data<Tera>) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);

    match tera.render("claims.html", &context) {
        Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
        Err(e) => {
            eprintln!("Template error: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(claims);
}
