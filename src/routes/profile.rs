use actix_web::{HttpResponse, get, post, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use tera::{Context, Tera};

use crate::middleware::auth::{Authenticated, UserCache};
use crate::models::error::is_unique_violation;
use crate::models::user::{AuthError, User};
use crate::routes::nav::insert_nav_context;
use crate::routes::render::render;

#[derive(Deserialize)]
pub struct ProfileForm {
    name: String,
    email: String,
    current_password: String,
    password: Option<String>,
    confirm_password: Option<String>,
}

#[derive(serde::Serialize)]
struct FormValues<'a> {
    name: &'a str,
    email: &'a str,
}

fn render_profile(
    tera: &Tera,
    user: &Authenticated,
    form_name: &str,
    form_email: &str,
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let form = FormValues {
        name: form_name,
        email: form_email,
    };
    let mut context = Context::new();
    context.insert("form", &form);
    context.insert("message", message);
    context.insert("message_kind", message_kind);
    insert_nav_context(&mut context, user);
    render(tera, "users/profile.html", &context)
}

/// Show the current user's profile form.
#[get("/profile")]
pub async fn show_profile(user: Authenticated, tera: web::Data<Tera>) -> HttpResponse {
    render_profile(tera.get_ref(), &user, &user.name, &user.email, "", "")
}

/// Update the current user's profile.
#[post("/profile")]
pub async fn update_profile(
    user: Authenticated,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    cache: web::Data<UserCache>,
    form: web::Form<ProfileForm>,
) -> HttpResponse {
    let name = form.name.trim();
    let email = form.email.trim();

    let existing = match User::find_by_id(db.get_ref(), user.id).await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().body("User not found."),
        Err(e) => {
            eprintln!("Failed to find user {}: {e}", user.id);
            return HttpResponse::InternalServerError().finish();
        }
    };

    // Require current password to authorize self-edit.
    if existing
        .verify_password(form.current_password.as_str())
        .is_err()
    {
        return render_profile(
            tera.get_ref(),
            &user,
            name,
            email,
            "Current password is incorrect.",
            "error",
        );
    }

    let mut errors = Vec::new();
    if name.is_empty() {
        errors.push("Name is required.");
    }
    if email.is_empty() {
        errors.push("Email is required.");
    } else if !email.contains('@') {
        errors.push("Email must contain '@'.");
    }

    let password_opt = form.password.as_deref().filter(|p| !p.trim().is_empty());
    if let Some(pw) = password_opt {
        let confirm = form.confirm_password.as_deref().unwrap_or_default().trim();
        if pw != confirm {
            errors.push("New password and confirmation do not match.");
        }
    }

    if !errors.is_empty() {
        return render_profile(
            tera.get_ref(),
            &user,
            name,
            email,
            &errors.join(" "),
            "error",
        );
    }

    match existing
        .update(
            db.get_ref(),
            cache.get_ref(),
            Some(email),
            Some(name),
            password_opt,
            None,
        )
        .await
    {
        Ok(_) => render_profile(
            tera.get_ref(),
            &user,
            name,
            email,
            "Profile updated. If you changed your email, you may need to log in again.",
            "success",
        ),
        Err(AuthError::DatabaseError(ref e)) if is_unique_violation(e) => render_profile(
            tera.get_ref(),
            &user,
            name,
            email,
            "That email is already in use.",
            "error",
        ),
        Err(e) => {
            eprintln!("Failed to update profile: {e}");
            render_profile(
                tera.get_ref(),
                &user,
                name,
                email,
                "Failed to update profile.",
                "error",
            )
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(show_profile).service(update_profile);
}
