use actix_web::{HttpResponse, get, post, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use tera::{Context, Tera};

use crate::entity::user::{Role, UserStatus};
use crate::middleware::auth::UserCache;
use crate::middleware::permissions::{AdminOnly, Require};
use crate::models::error::AuthError;
use crate::models::user::User;
use crate::models::util::is_unique_violation;
use crate::routes::nav::insert_nav_context;
use crate::routes::render::render;

const ROLE_LABELS: &[&str] = &["Admin", "Accountant", "Staff"];

#[derive(Deserialize)]
pub struct UserForm {
    name: String,
    email: String,
    role: String,
    password: Option<String>,
}

#[derive(Deserialize)]
pub struct UserFilter {
    q: Option<String>,
    role: Option<String>,
    status: Option<String>,
}

#[derive(serde::Serialize)]
struct FormValues<'a> {
    name: &'a str,
    email: &'a str,
    role: &'a str,
}

struct ListContext<'a> {
    tera: &'a Tera,
    user: &'a Require<AdminOnly>,
    users: &'a [User],
    q: &'a str,
    role: &'a str,
    status: &'a str,
    roles: &'a [&'a str],
    message: String,
    message_kind: &'a str,
}

fn render_users_list(ctx: ListContext) -> HttpResponse {
    let mut context = Context::new();
    context.insert("users", ctx.users);
    context.insert("q", ctx.q);
    context.insert("role", ctx.role);
    context.insert("status", ctx.status);
    context.insert("roles", ctx.roles);
    context.insert("message", &ctx.message);
    context.insert("message_kind", ctx.message_kind);
    insert_nav_context(&mut context, ctx.user);
    render(ctx.tera, "users/list.html", &context)
}

/// Reload the full user list and render it with a flash message.
async fn reload_users_list(
    tera: &Tera,
    user: &Require<AdminOnly>,
    db: &DatabaseConnection,
    verb: &str,
) -> HttpResponse {
    match User::list(db, None, None, None).await {
        Ok(users) => render_users_list(ListContext {
            roles: ROLE_LABELS,
            tera,
            user,
            users: &users,
            q: "",
            role: "",
            status: "",
            message: format!("User {verb}."),
            message_kind: "success",
        }),
        Err(e) => {
            eprintln!("Failed to list users after {verb}: {e}");
            render_users_list(ListContext {
                roles: ROLE_LABELS,
                tera,
                user,
                users: &[],
                q: "",
                role: "",
                status: "",
                message: format!("User {verb}, but failed to load list."),
                message_kind: "error",
            })
        }
    }
}

struct FormContext<'a> {
    tera: &'a Tera,
    user: &'a Require<AdminOnly>,
    mode: &'a str,
    existing_user: Option<&'a User>,
    form_name: &'a str,
    form_email: &'a str,
    form_role: &'a str,
    roles: &'a [&'a str],
    message: &'a str,
    message_kind: &'a str,
}

fn render_user_form(ctx: FormContext) -> HttpResponse {
    let mut context = Context::new();
    context.insert("mode", ctx.mode);
    context.insert("user", &ctx.existing_user);
    let form = FormValues {
        name: ctx.form_name,
        email: ctx.form_email,
        role: ctx.form_role,
    };
    context.insert("form", &form);
    context.insert("roles", ctx.roles);
    context.insert("message", ctx.message);
    context.insert("message_kind", ctx.message_kind);
    insert_nav_context(&mut context, ctx.user);
    render(ctx.tera, "users/form.html", &context)
}

/// List users with optional filters.
#[get("/users")]
pub async fn list_users(
    user: Require<AdminOnly>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<UserFilter>,
) -> HttpResponse {
    let q = query.q.as_deref();
    let role = query.role.as_deref().and_then(Role::parse);
    let status = query.status.as_deref().and_then(UserStatus::parse);

    match User::list(db.get_ref(), q, role, status).await {
        Ok(users) => render_users_list(ListContext {
            roles: ROLE_LABELS,
            tera: tera.get_ref(),
            user: &user,
            users: &users,
            q: q.unwrap_or_default(),
            role: query.role.as_deref().unwrap_or_default(),
            status: query.status.as_deref().unwrap_or_default(),
            message: String::new(),
            message_kind: "",
        }),
        Err(e) => {
            eprintln!("Failed to list users: {e}");
            render_users_list(ListContext {
                roles: ROLE_LABELS,
                tera: tera.get_ref(),
                user: &user,
                users: &[],
                q: q.unwrap_or_default(),
                role: query.role.as_deref().unwrap_or_default(),
                status: query.status.as_deref().unwrap_or_default(),
                message: "Failed to load users.".into(),
                message_kind: "error",
            })
        }
    }
}

/// Show the create-user form.
#[get("/users/new")]
pub async fn new_user(user: Require<AdminOnly>, tera: web::Data<Tera>) -> HttpResponse {
    render_user_form(FormContext {
        roles: ROLE_LABELS,
        tera: tera.get_ref(),
        user: &user,
        mode: "create",
        existing_user: None,
        form_name: "",
        form_email: "",
        form_role: "Staff",
        message: "",
        message_kind: "",
    })
}

/// Create a new user.
#[post("/users")]
pub async fn create_user(
    user: Require<AdminOnly>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    form: web::Form<UserForm>,
) -> HttpResponse {
    let name = form.name.trim();
    let email = form.email.trim();
    let role_str = form.role.trim();
    let password = form.password.as_deref().unwrap_or_default().trim();

    let mut errors = Vec::new();
    if name.is_empty() {
        errors.push("Name is required.");
    }
    if email.is_empty() {
        errors.push("Email is required.");
    } else if !email.contains('@') {
        errors.push("Email must contain '@'.");
    }
    if password.is_empty() {
        errors.push("Password is required.");
    }
    let role = match Role::parse(role_str) {
        Some(r) => r,
        None => {
            errors.push("Role must be Admin, Accountant, or Staff.");
            Role::Staff
        }
    };

    if !errors.is_empty() {
        return render_user_form(FormContext {
            roles: ROLE_LABELS,
            tera: tera.get_ref(),
            user: &user,
            mode: "create",
            existing_user: None,
            form_name: form.name.trim(),
            form_email: form.email.trim(),
            form_role: form.role.trim(),
            message: &errors.join(" "),
            message_kind: "error",
        });
    }

    match User::create(db.get_ref(), email, name, password, role).await {
        Ok(_) => reload_users_list(tera.get_ref(), &user, db.get_ref(), "created").await,
        Err(AuthError::Database(ref e)) if is_unique_violation(e) => {
            render_user_form(FormContext {
                roles: ROLE_LABELS,
                tera: tera.get_ref(),
                user: &user,
                mode: "create",
                existing_user: None,
                form_name: form.name.trim(),
                form_email: form.email.trim(),
                form_role: form.role.trim(),
                message: "A user with that email already exists.",
                message_kind: "error",
            })
        }
        Err(e) => {
            eprintln!("Failed to create user: {e}");
            render_user_form(FormContext {
                roles: ROLE_LABELS,
                tera: tera.get_ref(),
                user: &user,
                mode: "create",
                existing_user: None,
                form_name: form.name.trim(),
                form_email: form.email.trim(),
                form_role: form.role.trim(),
                message: "Failed to create user.",
                message_kind: "error",
            })
        }
    }
}

/// Show the edit-user form.
#[get("/users/{id}/edit")]
pub async fn edit_user(
    user: Require<AdminOnly>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    match User::find_by_id(db.get_ref(), id).await {
        Ok(Some(existing)) => render_user_form(FormContext {
            roles: ROLE_LABELS,
            tera: tera.get_ref(),
            user: &user,
            mode: "edit",
            existing_user: Some(&existing),
            form_name: &existing.name,
            form_email: &existing.email,
            form_role: &existing.role.to_string(),
            message: "",
            message_kind: "",
        }),
        Ok(None) => HttpResponse::NotFound().body("User not found."),
        Err(e) => {
            eprintln!("Failed to find user {id}: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Update an existing user.
#[post("/users/{id}")]
pub async fn update_user(
    current_user: Require<AdminOnly>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    cache: web::Data<UserCache>,
    path: web::Path<i32>,
    form: web::Form<UserForm>,
) -> HttpResponse {
    let id = path.into_inner();
    let name = form.name.trim();
    let email = form.email.trim();
    let role_str = form.role.trim();

    // Block changing own role
    if id == current_user.id
        && Role::parse(role_str) != Some(current_user.role.clone())
        && let Ok(Some(existing)) = User::find_by_id(db.get_ref(), id).await
    {
        return render_user_form(FormContext {
            roles: ROLE_LABELS,
            tera: tera.get_ref(),
            user: &current_user,
            mode: "edit",
            existing_user: Some(&existing),
            form_name: form.name.trim(),
            form_email: form.email.trim(),
            form_role: form.role.trim(),
            message: "You cannot change your own role.",
            message_kind: "error",
        });
    }

    let existing = match User::find_by_id(db.get_ref(), id).await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().body("User not found."),
        Err(e) => {
            eprintln!("Failed to find user {id}: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut errors = Vec::new();
    if name.is_empty() {
        errors.push("Name is required.");
    }
    if email.is_empty() {
        errors.push("Email is required.");
    } else if !email.contains('@') {
        errors.push("Email must contain '@'.");
    }
    let role = match Role::parse(role_str) {
        Some(r) => r,
        None => {
            errors.push("Role must be Admin, Accountant, or Staff.");
            Role::Staff
        }
    };

    if !errors.is_empty() {
        return render_user_form(FormContext {
            roles: ROLE_LABELS,
            tera: tera.get_ref(),
            user: &current_user,
            mode: "edit",
            existing_user: Some(&existing),
            form_name: form.name.trim(),
            form_email: form.email.trim(),
            form_role: form.role.trim(),
            message: &errors.join(" "),
            message_kind: "error",
        });
    }

    let password_opt = form.password.as_deref().filter(|p| !p.trim().is_empty());

    match existing
        .update(
            db.get_ref(),
            cache.get_ref(),
            Some(email),
            Some(name),
            password_opt,
            Some(role),
        )
        .await
    {
        Ok(_) => reload_users_list(tera.get_ref(), &current_user, db.get_ref(), "updated").await,
        Err(AuthError::Database(ref e)) if is_unique_violation(e) => {
            render_user_form(FormContext {
                roles: ROLE_LABELS,
                tera: tera.get_ref(),
                user: &current_user,
                mode: "edit",
                existing_user: Some(&existing),
                form_name: form.name.trim(),
                form_email: form.email.trim(),
                form_role: form.role.trim(),
                message: "That email is already in use.",
                message_kind: "error",
            })
        }
        Err(e) => {
            eprintln!("Failed to update user {id}: {e}");
            render_user_form(FormContext {
                roles: ROLE_LABELS,
                tera: tera.get_ref(),
                user: &current_user,
                mode: "edit",
                existing_user: Some(&existing),
                form_name: form.name.trim(),
                form_email: form.email.trim(),
                form_role: form.role.trim(),
                message: "Failed to update user.",
                message_kind: "error",
            })
        }
    }
}

/// Disable a user account.
#[post("/users/{id}/disable")]
pub async fn disable_user(
    current_user: Require<AdminOnly>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    cache: web::Data<UserCache>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    if id == current_user.id {
        let users = User::list(db.get_ref(), None, None, None)
            .await
            .unwrap_or_default();
        return render_users_list(ListContext {
            roles: ROLE_LABELS,
            tera: tera.get_ref(),
            user: &current_user,
            users: &users,
            q: "",
            role: "",
            status: "",
            message: "You cannot disable your own account.".into(),
            message_kind: "error",
        });
    }

    let existing = match User::find_by_id(db.get_ref(), id).await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().body("User not found."),
        Err(e) => {
            eprintln!("Failed to find user {id}: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match existing
        .set_disabled(db.get_ref(), cache.get_ref(), true)
        .await
    {
        Ok(_) => reload_users_list(tera.get_ref(), &current_user, db.get_ref(), "disabled").await,
        Err(e) => {
            eprintln!("Failed to disable user {id}: {e}");
            let users = User::list(db.get_ref(), None, None, None)
                .await
                .unwrap_or_default();
            render_users_list(ListContext {
                roles: ROLE_LABELS,
                tera: tera.get_ref(),
                user: &current_user,
                users: &users,
                q: "",
                role: "",
                status: "",
                message: "Failed to disable user.".into(),
                message_kind: "error",
            })
        }
    }
}

/// Enable a user account.
#[post("/users/{id}/enable")]
pub async fn enable_user(
    current_user: Require<AdminOnly>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    cache: web::Data<UserCache>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let existing = match User::find_by_id(db.get_ref(), id).await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().body("User not found."),
        Err(e) => {
            eprintln!("Failed to find user {id}: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match existing
        .set_disabled(db.get_ref(), cache.get_ref(), false)
        .await
    {
        Ok(_) => reload_users_list(tera.get_ref(), &current_user, db.get_ref(), "enabled").await,
        Err(e) => {
            eprintln!("Failed to enable user {id}: {e}");
            let users = User::list(db.get_ref(), None, None, None)
                .await
                .unwrap_or_default();
            render_users_list(ListContext {
                roles: ROLE_LABELS,
                tera: tera.get_ref(),
                user: &current_user,
                users: &users,
                q: "",
                role: "",
                status: "",
                message: "Failed to enable user.".into(),
                message_kind: "error",
            })
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_users)
        .service(new_user)
        .service(create_user)
        .service(edit_user)
        .service(update_user)
        .service(disable_user)
        .service(enable_user);
}
