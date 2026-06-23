use actix_web::{HttpResponse, Responder, get, web};
use tera::{Context, Tera};

use crate::middleware::permissions::{Finance, Require};
use crate::routes::nav::insert_nav_context;

#[get("/reports")]
pub async fn reports(user: Require<Finance>, tera: web::Data<Tera>) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);

    match tera.render("reports.html", &context) {
        Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
        Err(e) => {
            eprintln!("Template error: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(reports);
}
