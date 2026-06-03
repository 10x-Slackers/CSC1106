use actix_identity::Identity;
use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use tera::{Context, Tera};

use crate::models::user::find_user;

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[get("/login")]
pub async fn show_login(
    user: Option<Identity>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if user.is_some() {
        return HttpResponse::Found()
            .append_header(("Location", "/dashboard"))
            .finish();
    }

    render_login(tera.get_ref(), "")
}

#[post("/login")]
pub async fn process_login(
    req:  HttpRequest,
    form: web::Form<LoginForm>,
    tera: web::Data<Tera>,
) -> impl Responder {
    // TODO: Replace with hashed password check once you add password hashing
    match find_user(&form.username) {
        Some(user) if user.password == form.password => {
            Identity::login(&req.extensions(), user.username.to_string())
                .expect("Failed to create identity");

            HttpResponse::Found()
                .append_header(("Location", "/dashboard"))
                .finish()
        }
        _ => render_login(tera.get_ref(), "Wrong username or password."),
    }
}

#[post("/logout")]
pub async fn logout(user: Option<Identity>) -> impl Responder {
    if let Some(identity) = user {
        identity.logout();
    }

    HttpResponse::Found()
        .append_header(("Location", "/login"))
        .finish()
}

fn render_login(tera: &Tera, message: &str) -> HttpResponse {
    let mut context = Context::new();
    context.insert("message", message);

    let rendered = tera
        .render("login.html", &context)
        .expect("Failed to render login template");

    HttpResponse::Ok()
        .content_type("text/html")
        .body(rendered)
}
