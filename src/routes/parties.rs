//! Handles party management, managing party details.
//!
//! Authors: Tan Yong Meng

use actix_web::{HttpResponse, get, post, web};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};

use crate::entity::party::{PartyStatus, PartyType};
use crate::middleware::auth::Authenticated;
use crate::middleware::permissions::{Finance, Require};
use crate::models::error::AppError;
use crate::models::invoice::Invoice;
use crate::models::party::Party;
use crate::models::payment::Payment;
use crate::models::util::non_empty;
use crate::routes::utils::{
    Pagination, base_query_string, find_or_404, insert_nav_context, parse_page, render,
    require_non_empty, validate_email,
};

/// Form data used when creating or updating a party.
#[derive(Deserialize)]
pub struct PartyForm {
    party_type: String,
    name: String,
    company: Option<String>,
    email: String,
    phone: String,
    address: String,
}

/// Query parameters used to filter the party list.
///
/// These values are read from the URL query string on the party listing page.
#[derive(Deserialize, Serialize)]
pub struct PartyFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    q: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    party_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    page: Option<String>,
}

/// Renders the party listing page.
fn render_parties_list(
    tera: &Tera,
    user: &Authenticated,
    parties: &[Party],
    pagination: &Pagination,
    base_query: &str,
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let mut context = Context::new();
    context.insert("parties", parties);
    context.insert("party_types", &PartyType::labels());
    context.insert("statuses", &PartyStatus::labels());
    context.insert("pagination", pagination);
    context.insert("pagination_base_query", base_query);
    context.insert("message", message);
    context.insert("message_kind", message_kind);
    insert_nav_context(&mut context, user);
    render(tera, "parties/list.html", &context)
}

/// Reloads the party listing page after a create, update, or status change.
///
/// If the list cannot be loaded, an error message is displayed instead.
async fn reload_parties_list(
    tera: &Tera,
    user: &Authenticated,
    db: &DatabaseConnection,
    verb: &str,
) -> HttpResponse {
    match Party::search(db, None, None, None, 1).await {
        Ok((parties, total_pages, current_page)) => {
            let pagination = Pagination {
                current: current_page,
                total_pages,
            };
            render_parties_list(
                tera,
                user,
                &parties,
                &pagination,
                "",
                &format!("Party {verb}."),
                "success",
            )
        }
        Err(e) => {
            eprintln!("Failed to list parties after {verb}: {e}");
            let pagination = Pagination {
                current: 1,
                total_pages: 1,
            };
            render_parties_list(
                tera,
                user,
                &[],
                &pagination,
                "",
                &format!("Party {verb}, but failed to load list."),
                "error",
            )
        }
    }
}

/// Renders the create or edit party form.
///
/// The `mode` value controls whether the form is displayed for creating or
/// editing a party. When editing, `existing_party` should contain the current
/// party data.
fn render_party_form(
    tera: &Tera,
    user: &Authenticated,
    mode: &str,
    existing_party: Option<&Party>,
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let mut context = Context::new();
    context.insert("mode", mode);
    context.insert("party", &existing_party);
    context.insert("party_types", &PartyType::labels());
    context.insert("entity_label", "Party");
    context.insert("base_path", "/parties");
    if let Some(party) = existing_party {
        context.insert("id", &party.id);
    }
    context.insert("message", message);
    context.insert("message_kind", message_kind);
    insert_nav_context(&mut context, user);
    render(tera, "parties/form.html", &context)
}

/// Displays the party listing page.
///
/// Supports filtering by search query, party type, status, and page number.
/// When no status filter is provided, only active parties are shown by default.
#[get("/parties")]
pub async fn list_parties(
    user: Authenticated,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<PartyFilter>,
) -> HttpResponse {
    let filter = query.into_inner();
    let q = filter.q.as_deref();
    let party_type = filter.party_type.as_deref().and_then(PartyType::parse);
    // Default to active (hide disabled) when no status filter is sent.
    let status = match filter.status.as_deref() {
        None => Some(PartyStatus::Active),
        Some("") => None,
        Some(s) => PartyStatus::parse(s),
    };
    let page = parse_page(filter.page.as_deref());
    let base_query = base_query_string(&filter);
    let pagination_default = Pagination {
        current: 1,
        total_pages: 1,
    };
    match Party::search(db.get_ref(), q, party_type, status, page).await {
        Ok((parties, total_pages, current_page)) => {
            let pagination = Pagination {
                current: current_page,
                total_pages,
            };
            render_parties_list(
                tera.get_ref(),
                &user,
                &parties,
                &pagination,
                &base_query,
                "",
                "",
            )
        }
        Err(e) => {
            eprintln!("Failed to list parties: {e}");
            render_parties_list(
                tera.get_ref(),
                &user,
                &[],
                &pagination_default,
                &base_query,
                "Failed to load parties.",
                "error",
            )
        }
    }
}

/// Displays the form for creating a new party.
///
/// Access is restricted to users with finance permissions.
#[get("/parties/new")]
pub async fn new_party(user: Require<Finance>, tera: web::Data<Tera>) -> HttpResponse {
    render_party_form(tera.get_ref(), &user, "create", None, "", "")
}

/// Creates a new party from submitted form data.
#[post("/parties")]
pub async fn create_party(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    form: web::Form<PartyForm>,
) -> HttpResponse {
    let name = form.name.trim();
    let email = form.email.trim();
    let phone = form.phone.trim();
    let address = form.address.trim();
    let party_type_str = form.party_type.trim();
    let company = form.company.as_deref().and_then(non_empty);

    let mut errors: Vec<String> = Vec::new();
    require_non_empty(name, &mut errors, "Name");
    validate_email(email, &mut errors);
    require_non_empty(phone, &mut errors, "Phone");
    require_non_empty(address, &mut errors, "Address");
    let party_type = match PartyType::parse(party_type_str) {
        Some(pt) => pt,
        None => {
            errors.push("Party type must be Customer or Vendor.".into());
            PartyType::Customer
        }
    };

    if !errors.is_empty() {
        return render_party_form(
            tera.get_ref(),
            &user,
            "create",
            None,
            &errors.join(" "),
            "error",
        );
    }

    match Party::create(
        db.get_ref(),
        party_type,
        name,
        company,
        email,
        phone,
        address,
    )
    .await
    {
        Ok(_) => reload_parties_list(tera.get_ref(), &user, db.get_ref(), "created").await,
        Err(AppError::Duplicate) => render_party_form(
            tera.get_ref(),
            &user,
            "create",
            None,
            "A party with that email already exists.",
            "error",
        ),
        Err(e) => {
            eprintln!("Failed to create party: {e}");
            render_party_form(
                tera.get_ref(),
                &user,
                "create",
                None,
                "Failed to create party.",
                "error",
            )
        }
    }
}

/// Displays the edit form for an existing party.
///
/// Returns a 404 response if the party cannot be found.
#[get("/parties/{id}/edit")]
pub async fn edit_party(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    match find_or_404(Party::find_by_id(db.get_ref(), id), id, "Party").await {
        Ok(existing) => render_party_form(tera.get_ref(), &user, "edit", Some(&existing), "", ""),
        Err(resp) => resp,
    }
}

/// Updates an existing party from submitted form data.
#[post("/parties/{id}")]
pub async fn update_party(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    form: web::Form<PartyForm>,
) -> HttpResponse {
    let id = path.into_inner();
    let name = form.name.trim();
    let email = form.email.trim();
    let phone = form.phone.trim();
    let address = form.address.trim();
    let party_type_str = form.party_type.trim();
    let company = form.company.as_deref().and_then(non_empty);

    let existing = match find_or_404(Party::find_by_id(db.get_ref(), id), id, "Party").await {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    let mut errors: Vec<String> = Vec::new();
    require_non_empty(name, &mut errors, "Name");
    validate_email(email, &mut errors);
    require_non_empty(phone, &mut errors, "Phone");
    require_non_empty(address, &mut errors, "Address");
    let party_type = match PartyType::parse(party_type_str) {
        Some(pt) => pt,
        None => {
            errors.push("Party type must be Customer or Vendor.".into());
            PartyType::Customer
        }
    };

    if !errors.is_empty() {
        return render_party_form(
            tera.get_ref(),
            &user,
            "edit",
            Some(&existing),
            &errors.join(" "),
            "error",
        );
    }

    match existing
        .update(
            db.get_ref(),
            Some(party_type),
            Some(name),
            Some(company),
            Some(email),
            Some(phone),
            Some(address),
        )
        .await
    {
        Ok(_) => reload_parties_list(tera.get_ref(), &user, db.get_ref(), "updated").await,
        Err(AppError::Duplicate) => render_party_form(
            tera.get_ref(),
            &user,
            "edit",
            Some(&existing),
            "That email is already in use.",
            "error",
        ),
        Err(e) => {
            eprintln!("Failed to update party {id}: {e}");
            render_party_form(
                tera.get_ref(),
                &user,
                "edit",
                Some(&existing),
                "Failed to update party.",
                "error",
            )
        }
    }
}

/// Deactivates an existing party.
#[post("/parties/{id}/deactivate")]
pub async fn deactivate_party(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let existing = match find_or_404(Party::find_by_id(db.get_ref(), id), id, "Party").await {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    match existing
        .set_status(db.get_ref(), PartyStatus::Inactive)
        .await
    {
        Ok(_) => reload_parties_list(tera.get_ref(), &user, db.get_ref(), "deactivated").await,
        Err(e) => {
            eprintln!("Failed to deactivate party {id}: {e}");
            let (parties, total_pages, current_page) =
                Party::search(db.get_ref(), None, None, None, 1)
                    .await
                    .unwrap_or_default();
            let pagination = Pagination {
                current: current_page,
                total_pages,
            };
            render_parties_list(
                tera.get_ref(),
                &user,
                &parties,
                &pagination,
                "",
                "Failed to deactivate party.",
                "error",
            )
        }
    }
}

/// Activates an existing party.
#[post("/parties/{id}/activate")]
pub async fn activate_party(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let existing = match find_or_404(Party::find_by_id(db.get_ref(), id), id, "Party").await {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    match existing.set_status(db.get_ref(), PartyStatus::Active).await {
        Ok(_) => reload_parties_list(tera.get_ref(), &user, db.get_ref(), "activated").await,
        Err(e) => {
            eprintln!("Failed to activate party {id}: {e}");
            let (parties, total_pages, current_page) =
                Party::search(db.get_ref(), None, None, None, 1)
                    .await
                    .unwrap_or_default();
            let pagination = Pagination {
                current: current_page,
                total_pages,
            };
            render_parties_list(
                tera.get_ref(),
                &user,
                &parties,
                &pagination,
                "",
                "Failed to activate party.",
                "error",
            )
        }
    }
}

/// Displays the details page for a party.
#[get("/parties/{id}")]
pub async fn show_party(
    user: Authenticated,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let party = match find_or_404(Party::find_by_id(db.get_ref(), id), id, "Party").await {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    let (invoice_count, total_spending, recent_payments) = {
        let db = db.get_ref();
        let count_fut = Invoice::count_for_party(db, id);
        let total_fut = Payment::total_for_party(db, id);
        let recent_fut = Payment::list(db, None, Some(id), Some(5));
        match futures::try_join!(count_fut, total_fut, recent_fut) {
            Ok((c, t, r)) => (c, t.to_string(), r),
            Err(e) => {
                eprintln!("Failed to load party stats for {id}: {e}");
                (0u64, Decimal::ZERO.to_string(), Vec::new())
            }
        }
    };

    let mut context = Context::new();
    context.insert("party", &party);
    context.insert("invoice_count", &invoice_count);
    context.insert("total_spending", &total_spending);
    context.insert("recent_payments", &recent_payments);
    context.insert("message", "");
    context.insert("message_kind", "");
    insert_nav_context(&mut context, &user);
    render(tera.get_ref(), "parties/show.html", &context)
}

/// Registers party routes with the Actix Web service configuration.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_parties)
        .service(new_party)
        .service(create_party)
        .service(edit_party)
        .service(update_party)
        .service(deactivate_party)
        .service(activate_party)
        .service(show_party);
}
