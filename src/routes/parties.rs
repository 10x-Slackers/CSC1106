use actix_web::{HttpResponse, get, post, web};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use tera::{Context, Tera};

use crate::entity::party::{PartyStatus, PartyType};
use crate::middleware::auth::Authenticated;
use crate::middleware::permissions::{Finance, Require};
use crate::models::error::AppError;
use crate::models::invoice::Invoice;
use crate::models::party::Party;
use crate::models::payment::Payment;
use crate::models::util::{is_unique_violation, non_empty};
use crate::routes::nav::insert_nav_context;
use crate::routes::render::render;

#[derive(Deserialize)]
pub struct PartyForm {
    party_type: String,
    name: String,
    company: Option<String>,
    email: String,
    phone: String,
    address: String,
}

#[derive(Deserialize)]
pub struct PartyFilter {
    q: Option<String>,
    party_type: Option<String>,
    status: Option<String>,
}

fn render_parties_list(
    tera: &Tera,
    user: &Authenticated,
    parties: &[Party],
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let mut context = Context::new();
    context.insert("parties", parties);
    context.insert("party_types", &PartyType::labels());
    context.insert("statuses", &PartyStatus::labels());
    context.insert("message", message);
    context.insert("message_kind", message_kind);
    insert_nav_context(&mut context, user);
    render(tera, "parties/list.html", &context)
}

/// Reload the full party list and render it with a flash message.
async fn reload_parties_list(
    tera: &Tera,
    user: &Authenticated,
    db: &DatabaseConnection,
    verb: &str,
) -> HttpResponse {
    match Party::list(db, None, None, None).await {
        Ok(parties) => {
            render_parties_list(tera, user, &parties, &format!("Party {verb}."), "success")
        }
        Err(e) => {
            eprintln!("Failed to list parties after {verb}: {e}");
            render_parties_list(
                tera,
                user,
                &[],
                &format!("Party {verb}, but failed to load list."),
                "error",
            )
        }
    }
}

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

/// List parties with optional filters.
#[get("/parties")]
pub async fn list_parties(
    user: Authenticated,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<PartyFilter>,
) -> HttpResponse {
    let q = query.q.as_deref();
    let party_type = query.party_type.as_deref().and_then(PartyType::parse);
    // Default to active (hide disabled) when no status filter is sent.
    let status = match query.status.as_deref() {
        None => Some(PartyStatus::Active),
        Some("") => None,
        Some(s) => PartyStatus::parse(s),
    };
    match Party::list(db.get_ref(), q, party_type, status).await {
        Ok(parties) => render_parties_list(tera.get_ref(), &user, &parties, "", ""),
        Err(e) => {
            eprintln!("Failed to list parties: {e}");
            render_parties_list(
                tera.get_ref(),
                &user,
                &[],
                "Failed to load parties.",
                "error",
            )
        }
    }
}

/// Show the create-party form.
#[get("/parties/new")]
pub async fn new_party(user: Require<Finance>, tera: web::Data<Tera>) -> HttpResponse {
    render_party_form(tera.get_ref(), &user, "create", None, "", "")
}

/// Create a new party.
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

    let mut errors = Vec::new();
    if name.is_empty() {
        errors.push("Name is required.");
    }
    if email.is_empty() {
        errors.push("Email is required.");
    } else if !email.contains('@') {
        errors.push("Email must contain '@'.");
    }
    if phone.is_empty() {
        errors.push("Phone is required.");
    }
    if address.is_empty() {
        errors.push("Address is required.");
    }
    let party_type = match PartyType::parse(party_type_str) {
        Some(pt) => pt,
        None => {
            errors.push("Party type must be Customer or Vendor.");
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
        Err(AppError::Database(ref e)) if is_unique_violation(e) => render_party_form(
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

/// Show the edit-party form.
#[get("/parties/{id}/edit")]
pub async fn edit_party(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    match Party::find_by_id(db.get_ref(), id).await {
        Ok(Some(existing)) => {
            render_party_form(tera.get_ref(), &user, "edit", Some(&existing), "", "")
        }
        Ok(None) => HttpResponse::NotFound().body("Party not found."),
        Err(e) => {
            eprintln!("Failed to find party {id}: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Update an existing party.
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

    let existing = match Party::find_by_id(db.get_ref(), id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Party not found."),
        Err(e) => {
            eprintln!("Failed to find party {id}: {e}");
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
    if phone.is_empty() {
        errors.push("Phone is required.");
    }
    if address.is_empty() {
        errors.push("Address is required.");
    }
    let party_type = match PartyType::parse(party_type_str) {
        Some(pt) => pt,
        None => {
            errors.push("Party type must be Customer or Vendor.");
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
        Err(AppError::Database(ref e)) if is_unique_violation(e) => render_party_form(
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

/// Deactivate a party.
#[post("/parties/{id}/deactivate")]
pub async fn deactivate_party(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let existing = match Party::find_by_id(db.get_ref(), id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Party not found."),
        Err(e) => {
            eprintln!("Failed to find party {id}: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match existing
        .set_status(db.get_ref(), PartyStatus::Inactive)
        .await
    {
        Ok(_) => reload_parties_list(tera.get_ref(), &user, db.get_ref(), "deactivated").await,
        Err(e) => {
            eprintln!("Failed to deactivate party {id}: {e}");
            let parties = Party::list(db.get_ref(), None, None, None)
                .await
                .unwrap_or_default();
            render_parties_list(
                tera.get_ref(),
                &user,
                &parties,
                "Failed to deactivate party.",
                "error",
            )
        }
    }
}

/// Activate a party.
#[post("/parties/{id}/activate")]
pub async fn activate_party(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let existing = match Party::find_by_id(db.get_ref(), id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Party not found."),
        Err(e) => {
            eprintln!("Failed to find party {id}: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match existing.set_status(db.get_ref(), PartyStatus::Active).await {
        Ok(_) => reload_parties_list(tera.get_ref(), &user, db.get_ref(), "activated").await,
        Err(e) => {
            eprintln!("Failed to activate party {id}: {e}");
            let parties = Party::list(db.get_ref(), None, None, None)
                .await
                .unwrap_or_default();
            render_parties_list(
                tera.get_ref(),
                &user,
                &parties,
                "Failed to activate party.",
                "error",
            )
        }
    }
}

/// Party dashboard / detail view.
#[get("/parties/{id}")]
pub async fn show_party(
    user: Authenticated,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let party = match Party::find_by_id(db.get_ref(), id).await {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Party not found."),
        Err(e) => {
            eprintln!("Failed to find party {id}: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let invoice_count = match Invoice::count_for_party(db.get_ref(), id).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load invoice count for party {id}: {e}");
            0
        }
    };

    let total_spending = match Payment::total_for_party(db.get_ref(), id).await {
        Ok(total) => total.to_string(),
        Err(e) => {
            eprintln!("Failed to load payment total for party {id}: {e}");
            Decimal::ZERO.to_string()
        }
    };

    let recent_payments = match Payment::recent_for_party(db.get_ref(), id, 5).await {
        Ok(payments) => payments,
        Err(e) => {
            eprintln!("Failed to load recent payments for party {id}: {e}");
            Vec::new()
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
