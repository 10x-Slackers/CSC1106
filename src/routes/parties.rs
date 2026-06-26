use actix_web::{HttpResponse, get, post, web};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use tera::{Context, Tera};

use crate::entity::party::{PartyStatus, PartyType};
use crate::middleware::auth::Authenticated;
use crate::middleware::permissions::{Finance, Require};
use crate::models::error::is_unique_violation;
use crate::models::party::Party;
use crate::models::user::AuthError;
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

#[derive(serde::Serialize)]
struct FormValues<'a> {
    name: &'a str,
    company: Option<&'a str>,
    email: &'a str,
    phone: &'a str,
    address: &'a str,
    party_type: &'a str,
}

struct ListContext<'a> {
    tera: &'a Tera,
    user: &'a Authenticated,
    parties: &'a [Party],
    q: &'a str,
    party_type: &'a str,
    status: &'a str,
    message: String,
    message_kind: &'a str,
}

fn render_parties_list(ctx: ListContext) -> HttpResponse {
    let mut context = Context::new();
    context.insert("parties", ctx.parties);
    context.insert("q", ctx.q);
    context.insert("party_type", ctx.party_type);
    context.insert("status", ctx.status);
    context.insert("message", &ctx.message);
    context.insert("message_kind", ctx.message_kind);
    insert_nav_context(&mut context, ctx.user);
    render(ctx.tera, "parties/list.html", &context)
}

/// Reload the full party list and render it with a flash message.
async fn reload_parties_list(
    tera: &Tera,
    user: &Authenticated,
    db: &DatabaseConnection,
    verb: &str,
) -> HttpResponse {
    match Party::list(db, None, None, None).await {
        Ok(parties) => render_parties_list(ListContext {
            tera,
            user,
            parties: &parties,
            q: "",
            party_type: "",
            status: "",
            message: format!("Party {verb}."),
            message_kind: "success",
        }),
        Err(e) => {
            eprintln!("Failed to list parties after {verb}: {e}");
            render_parties_list(ListContext {
                tera,
                user,
                parties: &[],
                q: "",
                party_type: "",
                status: "",
                message: format!("Party {verb}, but failed to load list."),
                message_kind: "error",
            })
        }
    }
}

struct FormContext<'a> {
    tera: &'a Tera,
    user: &'a Authenticated,
    mode: &'a str,
    existing_party: Option<&'a Party>,
    form_name: &'a str,
    form_company: Option<&'a str>,
    form_email: &'a str,
    form_phone: &'a str,
    form_address: &'a str,
    form_party_type: &'a str,
    message: &'a str,
    message_kind: &'a str,
}

fn render_party_form(ctx: FormContext) -> HttpResponse {
    let mut context = Context::new();
    context.insert("mode", ctx.mode);
    context.insert("party", &ctx.existing_party);
    let form = FormValues {
        name: ctx.form_name,
        company: ctx.form_company,
        email: ctx.form_email,
        phone: ctx.form_phone,
        address: ctx.form_address,
        party_type: ctx.form_party_type,
    };
    context.insert("form", &form);
    context.insert("message", ctx.message);
    context.insert("message_kind", ctx.message_kind);
    insert_nav_context(&mut context, ctx.user);
    render(ctx.tera, "parties/form.html", &context)
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
    let status_str = match query.status.as_deref() {
        Some(s) if PartyStatus::parse(s).is_some() => s,
        _ => "Active",
    };

    match Party::list(db.get_ref(), q, party_type, status).await {
        Ok(parties) => render_parties_list(ListContext {
            tera: tera.get_ref(),
            user: &user,
            parties: &parties,
            q: q.unwrap_or_default(),
            party_type: query.party_type.as_deref().unwrap_or_default(),
            status: status_str,
            message: String::new(),
            message_kind: "",
        }),
        Err(e) => {
            eprintln!("Failed to list parties: {e}");
            render_parties_list(ListContext {
                tera: tera.get_ref(),
                user: &user,
                parties: &[],
                q: q.unwrap_or_default(),
                party_type: query.party_type.as_deref().unwrap_or_default(),
                status: status_str,
                message: "Failed to load parties.".into(),
                message_kind: "error",
            })
        }
    }
}

/// Show the create-party form.
#[get("/parties/new")]
pub async fn new_party(user: Require<Finance>, tera: web::Data<Tera>) -> HttpResponse {
    render_party_form(FormContext {
        tera: tera.get_ref(),
        user: &user,
        mode: "create",
        existing_party: None,
        form_name: "",
        form_company: None,
        form_email: "",
        form_phone: "",
        form_address: "",
        form_party_type: "Customer",
        message: "",
        message_kind: "",
    })
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
    let company = form.company.as_deref().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

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
        return render_party_form(FormContext {
            tera: tera.get_ref(),
            user: &user,
            mode: "create",
            existing_party: None,
            form_name: form.name.trim(),
            form_company: company,
            form_email: form.email.trim(),
            form_phone: form.phone.trim(),
            form_address: form.address.trim(),
            form_party_type: form.party_type.trim(),
            message: &errors.join(" "),
            message_kind: "error",
        });
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
        Err(AuthError::DatabaseError(ref e)) if is_unique_violation(e) => {
            render_party_form(FormContext {
                tera: tera.get_ref(),
                user: &user,
                mode: "create",
                existing_party: None,
                form_name: form.name.trim(),
                form_company: company,
                form_email: form.email.trim(),
                form_phone: form.phone.trim(),
                form_address: form.address.trim(),
                form_party_type: form.party_type.trim(),
                message: "A party with that email already exists.",
                message_kind: "error",
            })
        }
        Err(e) => {
            eprintln!("Failed to create party: {e}");
            render_party_form(FormContext {
                tera: tera.get_ref(),
                user: &user,
                mode: "create",
                existing_party: None,
                form_name: form.name.trim(),
                form_company: company,
                form_email: form.email.trim(),
                form_phone: form.phone.trim(),
                form_address: form.address.trim(),
                form_party_type: form.party_type.trim(),
                message: "Failed to create party.",
                message_kind: "error",
            })
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
        Ok(Some(existing)) => render_party_form(FormContext {
            tera: tera.get_ref(),
            user: &user,
            mode: "edit",
            existing_party: Some(&existing),
            form_name: &existing.name,
            form_company: existing.company.as_deref(),
            form_email: &existing.email,
            form_phone: &existing.phone,
            form_address: &existing.address,
            form_party_type: &existing.party_type.to_string(),
            message: "",
            message_kind: "",
        }),
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
    let company = form.company.as_deref().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

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
        return render_party_form(FormContext {
            tera: tera.get_ref(),
            user: &user,
            mode: "edit",
            existing_party: Some(&existing),
            form_name: form.name.trim(),
            form_company: company,
            form_email: form.email.trim(),
            form_phone: form.phone.trim(),
            form_address: form.address.trim(),
            form_party_type: form.party_type.trim(),
            message: &errors.join(" "),
            message_kind: "error",
        });
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
        Err(AuthError::DatabaseError(ref e)) if is_unique_violation(e) => {
            render_party_form(FormContext {
                tera: tera.get_ref(),
                user: &user,
                mode: "edit",
                existing_party: Some(&existing),
                form_name: form.name.trim(),
                form_company: company,
                form_email: form.email.trim(),
                form_phone: form.phone.trim(),
                form_address: form.address.trim(),
                form_party_type: form.party_type.trim(),
                message: "That email is already in use.",
                message_kind: "error",
            })
        }
        Err(e) => {
            eprintln!("Failed to update party {id}: {e}");
            render_party_form(FormContext {
                tera: tera.get_ref(),
                user: &user,
                mode: "edit",
                existing_party: Some(&existing),
                form_name: form.name.trim(),
                form_company: company,
                form_email: form.email.trim(),
                form_phone: form.phone.trim(),
                form_address: form.address.trim(),
                form_party_type: form.party_type.trim(),
                message: "Failed to update party.",
                message_kind: "error",
            })
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
            render_parties_list(ListContext {
                tera: tera.get_ref(),
                user: &user,
                parties: &parties,
                q: "",
                party_type: "",
                status: "",
                message: "Failed to deactivate party.".into(),
                message_kind: "error",
            })
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
            render_parties_list(ListContext {
                tera: tera.get_ref(),
                user: &user,
                parties: &parties,
                q: "",
                party_type: "",
                status: "",
                message: "Failed to activate party.".into(),
                message_kind: "error",
            })
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

    // TODO: replace with Invoice::count_for_party(db, id) once the invoice model lands
    let invoice_count: u64 = 0;

    // TODO: replace with Payment::total_for_party(db, id) once the payment model lands
    let total_spending = Decimal::ZERO.to_string();

    // TODO: replace with Payment::recent_for_party(db, id, 5) once the payment model lands
    let recent_payments: Vec<tera::Value> = Vec::new();

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
