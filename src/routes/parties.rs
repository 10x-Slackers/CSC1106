use actix_web::{Responder, get, web};
use tera::Context;

use crate::middleware::auth::Authenticated;
use crate::routes::nav::insert_nav_context;
use crate::routes::render::render;

#[get("/parties")]
pub async fn parties(user: Authenticated, tera: web::Data<tera::Tera>) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);
    render(&tera, "parties/index.html", &context)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(parties);
}
