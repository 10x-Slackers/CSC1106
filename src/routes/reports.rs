use actix_web::{Responder, get, web};
use tera::Context;

use crate::middleware::permissions::{Finance, Require};
use crate::routes::utils::{insert_nav_context, render};

#[get("/reports")]
pub async fn reports(user: Require<Finance>, tera: web::Data<tera::Tera>) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);
    render(&tera, "reports/index.html", &context)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(reports);
}
