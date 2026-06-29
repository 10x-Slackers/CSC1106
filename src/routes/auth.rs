use actix_identity::Identity;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, get, post, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use tera::{Context, Tera};

use crate::middleware::auth::UserCache;
use crate::models::error::AuthError;
use crate::models::user::User;
use crate::routes::utils::{redirect, render};

/// Form data for the login page.
#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

/// Render the login page; redirects to home if already authenticated.
#[get("/login")]
pub async fn show_login(user: Option<Identity>, tera: web::Data<Tera>) -> impl Responder {
    if user.is_some() {
        return redirect("/");
    }

    render_login(tera.get_ref(), "")
}

/// Process a login form submission and establish a session.
#[post("/login")]
pub async fn process_login(
    req: HttpRequest,
    form: web::Form<LoginForm>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    cache: web::Data<UserCache>,
) -> impl Responder {
    match User::authenticate(db.get_ref(), &form.email, &form.password).await {
        Ok(user) => {
            // Set session cookie, use email as identity
            if Identity::login(&req.extensions(), user.email.clone()).is_err() {
                return HttpResponse::InternalServerError().finish();
            }

            cache.insert(&user);

            redirect("/")
        }
        Err(AuthError::InvalidCredentials) => {
            render_login(tera.get_ref(), "Wrong email or password.")
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

/// Log out the current user and invalidate their cache entry.
#[post("/logout")]
pub async fn logout(user: Option<Identity>, cache: web::Data<UserCache>) -> impl Responder {
    if let Some(identity) = user {
        if let Ok(email) = identity.id() {
            cache.invalidate(&email);
        }
        identity.logout();
    }

    redirect("/login")
}

/// Helper function to render the login template with an optional status message.
fn render_login(tera: &Tera, message: &str) -> HttpResponse {
    let mut context = Context::new();
    context.insert("message", message);
    render(tera, "auth/login.html", &context)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(show_login)
        .service(process_login)
        .service(logout);
}
