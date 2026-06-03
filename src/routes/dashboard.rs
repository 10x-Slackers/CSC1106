use actix_identity::Identity;
use actix_web::{get, web, HttpResponse, Responder};
use tera::{Context, Tera};

use crate::models::user::find_user;

#[get("/dashboard")]
pub async fn dashboard(
    user: Option<Identity>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match user {
        Some(identity) => {
            let username = identity.id().unwrap_or_else(|_| "unknown".to_string());

            let display_name = find_user(&username)
                .map(|u| u.display_name.to_string())
                .unwrap_or(username);

            let mut context = Context::new();
            context.insert("username", &display_name);

            let rendered = tera
                .render("dashboard.html", &context)
                .expect("Failed to render dashboard template");

            HttpResponse::Ok()
                .content_type("text/html")
                .body(rendered)
        }
        None => HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish(),
    }
}
