use actix_web::{HttpResponse, get, post, web};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use tera::{Context, Tera};

use crate::entity::user::Role;
use crate::middleware::auth::Authenticated;
use crate::middleware::permissions::{Finance, Require, RoleSet};
use crate::models::claim::{Claim, ClaimFilter, ClaimForm, ClaimStats};
use crate::models::claim_category;
use crate::models::error::ClaimError;
use crate::routes::utils::{
    Pagination, base_query_string, insert_nav_context, parse_page, redirect, render,
    require_non_empty,
};

/// Form data submitted when rejecting a claim.
#[derive(Deserialize)]
struct RejectForm {
    #[serde(default)]
    rejection_reason: String,
}

/// Renders the claim details page.
///
/// The page displays a single claim together with an optional feedback message.
/// If the claim cannot be found, the user is redirected back to the claims list.
async fn render_claim_show(
    tera: &Tera,
    user: &Authenticated,
    db: &DatabaseConnection,
    id: i32,
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let detail = match Claim::find_with_linked(db, id).await {
        Ok(d) => d,
        Err(ClaimError::NotFound) => return redirect("/claims"),
        Err(e) => {
            eprintln!("Failed to find claim {id}: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut ctx = Context::new();
    ctx.insert("claim", &detail);
    ctx.insert("message", message);
    ctx.insert("message_kind", message_kind);
    insert_nav_context(&mut ctx, user);
    render(tera, "claims/show.html", &ctx)
}

/// Renders the claim listing page.
///
/// Staff users only see their own claims, while finance users can view all
/// claims and see claim statistics.
async fn render_claims_list(
    tera: &Tera,
    user: &Authenticated,
    db: &DatabaseConnection,
    filter: &ClaimFilter,
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let categories = claim_category::list_options(db).await.unwrap_or_default();

    let scope_user_id = if user.role == Role::Staff {
        Some(user.id)
    } else {
        None
    };

    let stats = if Finance::ROLES.contains(&user.role) {
        match ClaimStats::compute(db).await {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("Failed to compute claim stats: {e}");
                None
            }
        }
    } else {
        None
    };

    let page = parse_page(filter.page.as_deref());
    let mut list_filter = filter.clone();
    list_filter.page = Some(page.to_string());

    let (rows, total_pages, current_page) =
        match Claim::search(db, &list_filter, scope_user_id).await {
            Ok((r, tp, cp)) => (r, tp, cp),
            Err(e) => {
                eprintln!("Failed to list claims: {e}");
                (Vec::new(), 1, 1)
            }
        };

    let pagination = Pagination {
        current: current_page,
        total_pages,
    };
    let base_query = base_query_string(filter);

    let mut ctx = Context::new();
    ctx.insert("claims", &rows);
    ctx.insert("categories", &categories);
    ctx.insert("pagination", &pagination);
    ctx.insert("pagination_base_query", &base_query);
    ctx.insert("filter", filter);
    ctx.insert("message", message);
    ctx.insert("message_kind", message_kind);
    if let Some(ref s) = stats {
        ctx.insert("stats", s);
    }
    insert_nav_context(&mut ctx, user);
    render(tera, "claims/list.html", &ctx)
}

/// Renders the claim submission form.
///
/// Claim categories are loaded and inserted into the template context so the
/// user can select a category when creating a claim.
async fn render_claim_form(
    tera: &Tera,
    user: &Authenticated,
    db: &DatabaseConnection,
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let categories = claim_category::list_options(db).await.unwrap_or_default();

    let mut ctx = Context::new();
    ctx.insert("categories", &categories);
    ctx.insert("message", message);
    ctx.insert("message_kind", message_kind);
    insert_nav_context(&mut ctx, user);
    render(tera, "claims/form.html", &ctx)
}

/// Displays the claim listing page.
///
/// Supports filtering and pagination through [`ClaimFilter`].
#[get("/claims")]
pub async fn list_claims(
    user: Authenticated,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<ClaimFilter>,
) -> HttpResponse {
    render_claims_list(tera.get_ref(), &user, db.get_ref(), &query, "", "").await
}

/// Displays the form for submitting a new claim.
#[get("/claims/new")]
pub async fn new_claim(
    user: Authenticated,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
) -> HttpResponse {
    render_claim_form(tera.get_ref(), &user, db.get_ref(), "", "").await
}

/// Creates a new claim from submitted form data.
///
/// The form input is validated before the claim is saved. If validation fails,
/// the claim form is re-rendered with an error message.
#[post("/claims")]
pub async fn create_claim(
    user: Authenticated,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    form: web::Form<ClaimForm>,
) -> HttpResponse {
    let title = form.title.trim();
    let description = form.description.trim();
    let amount_str = form.amount.trim();
    let purchase_date_str = form.purchase_date.trim();

    let mut errors: Vec<String> = Vec::new();

    require_non_empty(title, &mut errors, "Title");
    require_non_empty(description, &mut errors, "Description");

    if title.len() > 255 {
        errors.push("Title must be at most 255 characters.".into());
    }

    if !amount_str
        .parse::<Decimal>()
        .map(|a| a > Decimal::ZERO)
        .unwrap_or(false)
    {
        errors.push("Amount must be a positive number.".into());
    }

    if NaiveDate::parse_from_str(purchase_date_str, "%Y-%m-%d").is_err() {
        errors.push("Purchase date is invalid.".into());
    }

    if !claim_category::exists(db.get_ref(), form.category_id)
        .await
        .unwrap_or(false)
    {
        errors.push("Selected category does not exist.".into());
    }

    if !errors.is_empty() {
        return render_claim_form(
            tera.get_ref(),
            &user,
            db.get_ref(),
            &errors.join(" "),
            "error",
        )
        .await;
    }

    let claim_form = ClaimForm {
        title: title.to_string(),
        category_id: form.category_id,
        description: description.to_string(),
        amount: amount_str.to_string(),
        purchase_date: purchase_date_str.to_string(),
    };

    match Claim::create(db.get_ref(), claim_form, user.id).await {
        Ok(model) => redirect(&format!("/claims/{}", model.id)),
        Err(e) => {
            eprintln!("Failed to create claim: {e}");
            render_claim_form(
                tera.get_ref(),
                &user,
                db.get_ref(),
                "Failed to submit claim.",
                "error",
            )
            .await
        }
    }
}

/// Displays the details page for a claim.
#[get("/claims/{id}")]
pub async fn show_claim(
    user: Authenticated,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();
    render_claim_show(tera.get_ref(), &user, db.get_ref(), id, "", "").await
}

/// Approves a pending claim.
///
/// Access is restricted to finance users. If the claim is no longer pending,
/// the claim details page is re-rendered with an error message.
#[post("/claims/{id}/approve")]
pub async fn approve_claim(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    match Claim::approve(db.get_ref(), id, user.id).await {
        Ok(_) => redirect(&format!("/claims/{id}")),
        Err(ClaimError::NotFound) => redirect("/claims"),
        Err(ClaimError::InvalidStatus) => {
            render_claim_show(
                tera.get_ref(),
                &user,
                db.get_ref(),
                id,
                "Claim is no longer pending.",
                "error",
            )
            .await
        }
        Err(e) => {
            eprintln!("Failed to approve claim {id}: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Rejects a pending claim with a rejection reason.
///
/// Access is restricted to finance users. A rejection reason is required
/// before the claim can be rejected.
#[post("/claims/{id}/reject")]
pub async fn reject_claim(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    form: web::Form<RejectForm>,
) -> HttpResponse {
    let id = path.into_inner();
    let reason = form.rejection_reason.trim();

    if reason.is_empty() {
        return render_claim_show(
            tera.get_ref(),
            &user,
            db.get_ref(),
            id,
            "Rejection reason is required.",
            "error",
        )
        .await;
    }

    match Claim::reject(db.get_ref(), id, user.id, reason.to_string()).await {
        Ok(_) => redirect(&format!("/claims/{id}")),
        Err(ClaimError::NotFound) => redirect("/claims"),
        Err(ClaimError::InvalidStatus) => {
            render_claim_show(
                tera.get_ref(),
                &user,
                db.get_ref(),
                id,
                "Claim is no longer pending.",
                "error",
            )
            .await
        }
        Err(e) => {
            eprintln!("Failed to reject claim {id}: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Withdraws the authenticated user's own pending claim.
///
/// Users can only withdraw claims they submitted themselves. Claims that are
/// no longer pending cannot be withdrawn.
#[post("/claims/{id}/withdraw")]
pub async fn withdraw_claim(
    user: Authenticated,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    match Claim::withdraw(db.get_ref(), id, user.id).await {
        Ok(()) => redirect("/claims"),
        Err(ClaimError::NotFound) => redirect("/claims"),
        Err(ClaimError::NotOwner) => {
            render_claim_show(
                tera.get_ref(),
                &user,
                db.get_ref(),
                id,
                "You can only withdraw your own claims.",
                "error",
            )
            .await
        }
        Err(ClaimError::InvalidStatus) => {
            render_claim_show(
                tera.get_ref(),
                &user,
                db.get_ref(),
                id,
                "Claim is no longer pending.",
                "error",
            )
            .await
        }
        Err(e) => {
            eprintln!("Failed to withdraw claim {id}: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Registers claim routes with the Actix Web service configuration.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_claims)
        .service(new_claim)
        .service(create_claim)
        .service(show_claim)
        .service(approve_claim)
        .service(reject_claim)
        .service(withdraw_claim);
}
