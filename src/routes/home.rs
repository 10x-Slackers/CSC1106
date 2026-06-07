use actix_web::{HttpResponse, Responder, get, web};

use crate::middleware::auth::Authenticated;

#[get("/")]
pub async fn home(user: Authenticated) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("Logged in as {} ({})", user.email, user.role))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(home);
}
