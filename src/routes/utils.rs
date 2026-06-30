use std::future::Future;
use std::str::FromStr;

use actix_web::HttpResponse;
use tera::{Context, Tera};

use crate::entity::user::Role;
use crate::middleware::auth::Authenticated;
use crate::middleware::permissions::{AdminOnly, Finance, RoleSet};

/// Render a Tera template into an HTML HttpResponse.
pub fn render(tera: &Tera, name: &str, context: &Context) -> HttpResponse {
    match tera.render(name, context) {
        Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
        Err(e) => {
            eprintln!("Template error: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// A navigation link.
#[derive(Clone, serde::Serialize)]
struct NavLink {
    name: String,
    href: String,
}

/// Insert navbar context variables into a Tera context.
pub fn insert_nav_context(context: &mut Context, user: &Authenticated) {
    context.insert("nav_links", &nav_links_for_role(&user.role));
    context.insert("current_user", user);
}

/// Build the navigation links for a given user role.
fn nav_links_for_role(role: &Role) -> Vec<NavLink> {
    let mut links = vec![
        NavLink {
            name: "Dashboard".into(),
            href: "/".into(),
        },
        NavLink {
            name: "Invoices".into(),
            href: "/invoices".into(),
        },
        NavLink {
            name: "Claims".into(),
            href: "/claims".into(),
        },
        NavLink {
            name: "Parties".into(),
            href: "/parties".into(),
        },
    ];

    if Finance::ROLES.contains(role) {
        links.push(NavLink {
            name: "Payments".into(),
            href: "/payments".into(),
        });
        links.push(NavLink {
            name: "Reports".into(),
            href: "/reports".into(),
        });
    }

    if AdminOnly::ROLES.contains(role) {
        links.push(NavLink {
            name: "Users".into(),
            href: "/users".into(),
        });
    }

    links
}

/// Build a 302 Found redirect response.
pub fn redirect(to: &str) -> HttpResponse {
    HttpResponse::Found()
        .append_header(("Location", to))
        .finish()
}

/// Resolve a `find_by_id`-style future into the entity, or a 404/500 response.
pub async fn find_or_404<T, E: std::fmt::Display>(
    future: impl Future<Output = Result<Option<T>, E>>,
    id: i32,
    label: &str,
) -> Result<T, HttpResponse> {
    match future.await {
        Ok(Some(entity)) => Ok(entity),
        Ok(None) => Err(HttpResponse::NotFound().body(format!("{label} not found."))),
        Err(e) => {
            eprintln!("Failed to find {label} {id}: {e}");
            Err(HttpResponse::InternalServerError().finish())
        }
    }
}

/// Push `"{field_name} is required."` into `errors` if `v` is empty.
pub fn require_non_empty(v: &str, errors: &mut Vec<String>, field_name: &str) {
    if v.is_empty() {
        errors.push(format!("{field_name} is required."));
    }
}

/// Validate an email: required + must contain '@'.
pub fn validate_email(email: &str, errors: &mut Vec<String>) {
    if email.is_empty() {
        errors.push("Email is required.".into());
    } else if !email.contains('@') {
        errors.push("Email must contain '@'.".into());
    }
}

/// Trim, parse, and on failure push `err_msg` and return `fallback`.
pub fn parse_field<T: FromStr>(v: &str, errors: &mut Vec<String>, err_msg: &str, fallback: T) -> T {
    match v.trim().parse() {
        Ok(val) => val,
        Err(_) => {
            errors.push(err_msg.into());
            fallback
        }
    }
}

#[derive(Clone, Copy, serde::Serialize)]
pub struct Pagination {
    pub current: u32,
    pub total_pages: u32,
}

pub fn parse_page(raw: Option<u32>) -> u32 {
    raw.unwrap_or(1).max(1)
}

/// Serialize `filter` to a query string, then strip any `page=...` pair.
pub fn base_query_string<T: serde::Serialize>(filter: &T) -> String {
    let qs = match serde_qs::to_string(filter) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };

    let stripped: Vec<&str> = qs
        .split('&')
        .filter(|pair| {
            let key = pair.split('=').next().unwrap_or("");
            key != "page"
        })
        .collect();

    if stripped.is_empty() {
        String::new()
    } else {
        stripped.join("&")
    }
}
