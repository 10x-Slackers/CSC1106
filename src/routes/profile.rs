//! Handles the current user’s own profile page.
//! Allows a logged-in user update their own account details safely.
//!
//! Authors: Tan Yong Meng

use actix_web::{HttpResponse, get, post, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use tera::{Context, Tera};

use crate::middleware::auth::{Authenticated, UserCache};
use crate::models::error::AuthError;
use crate::models::user::User;
use crate::routes::utils::{
    find_or_404, insert_nav_context, render, require_non_empty, validate_email,
};

/// Form data submitted from the profile update page.
#[derive(Deserialize)]
pub struct ProfileForm {
    name: String,
    email: String,
    current_password: String,
    password: Option<String>,
    confirm_password: Option<String>,
}

/// Form values used when rendering the profile form.
#[derive(serde::Serialize)]
struct FormValues<'a> {
    name: &'a str,
    email: &'a str,
}

/// Renders the profile page.
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

/// Displays the current user's profile page.
#[get("/profile")]
pub async fn show_profile(user: Authenticated, tera: web::Data<Tera>) -> HttpResponse {
    render_profile(tera.get_ref(), &user, &user.name, &user.email, "", "")
}

/// Updates the current user's profile.
///
/// The current password must be provided before changes are accepted. The name
/// and email are validated, and the password is updated only when a new
/// password is provided and matches the confirmation field.
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

    let existing = match find_or_404(User::find_by_id(db.get_ref(), user.id), user.id, "User").await
    {
        Ok(u) => u,
        Err(resp) => return resp,
    };

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

    let mut errors: Vec<String> = Vec::new();
    require_non_empty(name, &mut errors, "Name");
    validate_email(email, &mut errors);

    let password_opt = form.password.as_deref().filter(|p| !p.trim().is_empty());
    if let Some(pw) = password_opt {
        let confirm = form.confirm_password.as_deref().unwrap_or_default().trim();
        if pw != confirm {
            errors.push("New password and confirmation do not match.".into());
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
        .update(db.get_ref(), Some(email), Some(name), password_opt, None)
        .await
    {
        Ok(_) => {
            cache.invalidate(&existing.email);
            render_profile(
                tera.get_ref(),
                &user,
                name,
                email,
                "Profile updated. If you changed your email, you may need to log in again.",
                "success",
            )
        }
        Err(AuthError::Duplicate) => render_profile(
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

/// Registers profile routes with the Actix Web service configuration.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(show_profile).service(update_profile);
}
