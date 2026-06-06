use actix_identity::Identity;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, get, post, web};
use serde::Deserialize;
use tera::{Context, Tera};

use crate::models::user::{AuthError, authenticate};

#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

#[get("/login")]
pub async fn show_login(user: Option<Identity>, tera: web::Data<Tera>) -> impl Responder {
    if user.is_some() {
        return HttpResponse::Found()
            .append_header(("Location", "/"))
            .finish();
    }

    render_login(tera.get_ref(), "")
}

#[post("/login")]
pub async fn process_login(
    req: HttpRequest,
    form: web::Form<LoginForm>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match authenticate(&form.email, &form.password) {
        Ok(user) => {
            if Identity::login(&req.extensions(), user.email.clone()).is_err() {
                return HttpResponse::InternalServerError().finish();
            }

            HttpResponse::Found()
                .append_header(("Location", "/"))
                .finish()
        }
        Err(AuthError::InvalidCredentials) => {
            render_login(tera.get_ref(), "Wrong email or password.")
        }
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

    match tera.render("login.html", &context) {
        Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(show_login)
        .service(process_login)
        .service(logout);
}
