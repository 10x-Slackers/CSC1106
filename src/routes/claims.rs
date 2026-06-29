use actix_web::{Responder, get, web};
use tera::Context;

use crate::middleware::auth::Authenticated;
use crate::routes::utils::{insert_nav_context, render};

#[get("/claims")]
pub async fn claims(user: Authenticated, tera: web::Data<tera::Tera>) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);
    render(&tera, "claims/index.html", &context)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(claims);
}
