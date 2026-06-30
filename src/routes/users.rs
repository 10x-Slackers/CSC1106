use actix_web::{HttpResponse, get, post, web};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};

use crate::entity::user::{Role, UserStatus};
use crate::middleware::auth::UserCache;
use crate::middleware::permissions::{AdminOnly, Require};
use crate::models::error::AuthError;
use crate::models::user::User;
use crate::models::util::is_unique_violation;
use crate::routes::pagination::{Pagination, base_query_string, clamp_page, parse_page};
use crate::routes::utils::{
    find_or_404, insert_nav_context, render, require_non_empty, validate_email,
};

#[derive(Deserialize)]
pub struct UserForm {
    name: String,
    email: String,
    role: String,
    password: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct UserFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    q: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    page: Option<u32>,
}

fn render_users_list(
    tera: &Tera,
    user: &Require<AdminOnly>,
    users: &[User],
    pagination: &Pagination,
    base_query: &str,
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let mut context = Context::new();
    context.insert("users", users);
    context.insert("roles", &Role::labels());
    context.insert("pagination", pagination);
    context.insert("pagination_base_query", base_query);
    context.insert("message", message);
    context.insert("message_kind", message_kind);
    insert_nav_context(&mut context, user);
    render(tera, "users/list.html", &context)
}

/// Reload the full user list and render it with a flash message.
async fn reload_users_list(
    tera: &Tera,
    user: &Require<AdminOnly>,
    db: &DatabaseConnection,
    verb: &str,
) -> HttpResponse {
    let pagination = Pagination {
        current: 1,
        total_pages: 1,
    };
    match User::list(db, None, None, None, 0).await {
        Ok((users, _)) => render_users_list(
            tera,
            user,
            &users,
            &pagination,
            "",
            &format!("User {verb}."),
            "success",
        ),
        Err(e) => {
            eprintln!("Failed to list users after {verb}: {e}");
            render_users_list(
                tera,
                user,
                &[],
                &pagination,
                "",
                &format!("User {verb}, but failed to load list."),
                "error",
            )
        }
    }
}

fn render_user_form(
    tera: &Tera,
    user: &Require<AdminOnly>,
    mode: &str,
    existing_user: Option<&User>,
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let mut context = Context::new();
    context.insert("mode", mode);
    context.insert("user", &existing_user);
    context.insert("roles", &Role::labels());
    context.insert("entity_label", "User");
    context.insert("base_path", "/users");
    if let Some(user) = existing_user {
        context.insert("id", &user.id);
    }
    context.insert("message", message);
    context.insert("message_kind", message_kind);
    insert_nav_context(&mut context, user);
    render(tera, "users/form.html", &context)
}

/// List users with optional filters.
#[get("/users")]
pub async fn list_users(
    user: Require<AdminOnly>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<UserFilter>,
) -> HttpResponse {
    let filter = query.into_inner();
    let q = filter.q.as_deref();
    let role = filter.role.as_deref().and_then(Role::parse);
    let status = filter.status.as_deref().and_then(UserStatus::parse);
    let page = parse_page(filter.page);
    let base_query = base_query_string(&filter);
    let pagination_default = Pagination {
        current: page,
        total_pages: 1,
    };

    match User::list(db.get_ref(), q, role, status, (page - 1) as u64).await {
        Ok((users, total_pages)) => {
            let page = clamp_page(page, total_pages);
            let pagination = Pagination {
                current: page,
                total_pages,
            };
            render_users_list(
                tera.get_ref(),
                &user,
                &users,
                &pagination,
                &base_query,
                "",
                "",
            )
        }
        Err(e) => {
            eprintln!("Failed to list users: {e}");
            render_users_list(
                tera.get_ref(),
                &user,
                &[],
                &pagination_default,
                &base_query,
                "Failed to load users.",
                "error",
            )
        }
    }
}

/// Show the create-user form.
#[get("/users/new")]
pub async fn new_user(user: Require<AdminOnly>, tera: web::Data<Tera>) -> HttpResponse {
    render_user_form(tera.get_ref(), &user, "create", None, "", "")
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

    let mut errors: Vec<String> = Vec::new();
    require_non_empty(name, &mut errors, "Name");
    validate_email(email, &mut errors);
    require_non_empty(password, &mut errors, "Password");
    let role = match Role::parse(role_str) {
        Some(r) => r,
        None => {
            errors.push("Role must be Admin, Accountant, or Staff.".into());
            Role::Staff
        }
    };

    if !errors.is_empty() {
        return render_user_form(
            tera.get_ref(),
            &user,
            "create",
            None,
            &errors.join(" "),
            "error",
        );
    }

    match User::create(db.get_ref(), email, name, password, role).await {
        Ok(_) => reload_users_list(tera.get_ref(), &user, db.get_ref(), "created").await,
        Err(AuthError::Database(ref e)) if is_unique_violation(e) => render_user_form(
            tera.get_ref(),
            &user,
            "create",
            None,
            "A user with that email already exists.",
            "error",
        ),
        Err(e) => {
            eprintln!("Failed to create user: {e}");
            render_user_form(
                tera.get_ref(),
                &user,
                "create",
                None,
                "Failed to create user.",
                "error",
            )
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

    match find_or_404(User::find_by_id(db.get_ref(), id), id, "User").await {
        Ok(existing) => render_user_form(tera.get_ref(), &user, "edit", Some(&existing), "", ""),
        Err(resp) => resp,
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
        return render_user_form(
            tera.get_ref(),
            &current_user,
            "edit",
            Some(&existing),
            "You cannot change your own role.",
            "error",
        );
    }

    let existing = match find_or_404(User::find_by_id(db.get_ref(), id), id, "User").await {
        Ok(u) => u,
        Err(resp) => return resp,
    };

    let mut errors: Vec<String> = Vec::new();
    require_non_empty(name, &mut errors, "Name");
    validate_email(email, &mut errors);
    let role = match Role::parse(role_str) {
        Some(r) => r,
        None => {
            errors.push("Role must be Admin, Accountant, or Staff.".into());
            Role::Staff
        }
    };

    if !errors.is_empty() {
        return render_user_form(
            tera.get_ref(),
            &current_user,
            "edit",
            Some(&existing),
            &errors.join(" "),
            "error",
        );
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
        Err(AuthError::Database(ref e)) if is_unique_violation(e) => render_user_form(
            tera.get_ref(),
            &current_user,
            "edit",
            Some(&existing),
            "That email is already in use.",
            "error",
        ),
        Err(e) => {
            eprintln!("Failed to update user {id}: {e}");
            render_user_form(
                tera.get_ref(),
                &current_user,
                "edit",
                Some(&existing),
                "Failed to update user.",
                "error",
            )
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
        let (users, _) = User::list(db.get_ref(), None, None, None, 0)
            .await
            .unwrap_or_default();
        let pagination = Pagination {
            current: 1,
            total_pages: 1,
        };
        return render_users_list(
            tera.get_ref(),
            &current_user,
            &users,
            &pagination,
            "",
            "You cannot disable your own account.",
            "error",
        );
    }

    let existing = match find_or_404(User::find_by_id(db.get_ref(), id), id, "User").await {
        Ok(u) => u,
        Err(resp) => return resp,
    };

    match existing
        .set_disabled(db.get_ref(), cache.get_ref(), true)
        .await
    {
        Ok(_) => reload_users_list(tera.get_ref(), &current_user, db.get_ref(), "disabled").await,
        Err(e) => {
            eprintln!("Failed to disable user {id}: {e}");
            let (users, _) = User::list(db.get_ref(), None, None, None, 0)
                .await
                .unwrap_or_default();
            let pagination = Pagination {
                current: 1,
                total_pages: 1,
            };
            render_users_list(
                tera.get_ref(),
                &current_user,
                &users,
                &pagination,
                "",
                "Failed to disable user.",
                "error",
            )
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

    let existing = match find_or_404(User::find_by_id(db.get_ref(), id), id, "User").await {
        Ok(u) => u,
        Err(resp) => return resp,
    };

    match existing
        .set_disabled(db.get_ref(), cache.get_ref(), false)
        .await
    {
        Ok(_) => reload_users_list(tera.get_ref(), &current_user, db.get_ref(), "enabled").await,
        Err(e) => {
            eprintln!("Failed to enable user {id}: {e}");
            let (users, _) = User::list(db.get_ref(), None, None, None, 0)
                .await
                .unwrap_or_default();
            let pagination = Pagination {
                current: 1,
                total_pages: 1,
            };
            render_users_list(
                tera.get_ref(),
                &current_user,
                &users,
                &pagination,
                "",
                "Failed to enable user.",
                "error",
            )
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
