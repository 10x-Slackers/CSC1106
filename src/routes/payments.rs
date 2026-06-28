use actix_web::{HttpResponse, get, post, web};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};

use crate::entity::payment::PaymentDirection;
use crate::middleware::auth::Authenticated;
use crate::middleware::permissions::{Finance, Require};
use crate::models::account::list_accounts;
use crate::models::error::PaymentCreateError;
use crate::models::party::Party;
use crate::models::payment::Payment;
use crate::routes::nav::insert_nav_context;
use crate::routes::render::render;

#[derive(Deserialize)]
struct PaymentForm {
    payment_direction: String,
    amount: String,
    payment_date: String,
    party_id: Option<String>,
    from_account_id: String,
    to_account_id: String,
    remarks: Option<String>,
}

#[derive(Deserialize, Default)]
struct PaymentFilter {
    q: Option<String>,
    direction: Option<String>,
    party_id: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
}

#[derive(Serialize)]
struct PaymentRow {
    id: i32,
    payment_date: NaiveDate,
    payment_direction: String,
    party_name: Option<String>,
    remarks: Option<String>,
    amount: Decimal,
}

fn direction_str(dir: &PaymentDirection) -> &'static str {
    match dir {
        PaymentDirection::In => "IN",
        PaymentDirection::Out => "OUT",
    }
}

/// Add party names to payments for rendering in the list view.
fn enrich_payments(
    payments: &[Payment],
    party_map: &std::collections::HashMap<i32, String>,
) -> Vec<PaymentRow> {
    payments
        .iter()
        .map(|p| PaymentRow {
            id: p.id,
            payment_date: p.payment_date,
            payment_direction: direction_str(&p.payment_direction).to_string(),
            party_name: p.party_id.and_then(|pid| party_map.get(&pid).cloned()),
            remarks: p.remarks.clone(),
            amount: p.amount,
        })
        .collect()
}

fn non_empty(s: &str) -> Option<&str> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t) }
}

/// Load parties + party-name map and render the payments list.
async fn render_payments_list(
    tera: &Tera,
    user: &Authenticated,
    db: &DatabaseConnection,
    filter: &PaymentFilter,
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let parties = Party::list_all(db).await.unwrap_or_default();
    let party_map = Party::name_map(db).await.unwrap_or_default();
    let payments = match Payment::list(
        db,
        filter.q.as_deref().and_then(non_empty),
        filter
            .direction
            .as_deref()
            .and_then(PaymentDirection::parse),
        filter
            .party_id
            .as_deref()
            .and_then(|s| s.parse::<i32>().ok()),
        filter
            .from_date
            .as_deref()
            .and_then(|s| s.parse::<NaiveDate>().ok()),
        filter
            .to_date
            .as_deref()
            .and_then(|s| s.parse::<NaiveDate>().ok()),
    )
    .await
    {
        Ok(p) => enrich_payments(&p, &party_map),
        Err(e) => {
            eprintln!("Failed to list payments: {e}");
            Vec::new()
        }
    };

    let mut ctx = Context::new();
    ctx.insert("payments", &payments);
    ctx.insert("parties", &parties);
    ctx.insert("message", message);
    ctx.insert("message_kind", message_kind);
    insert_nav_context(&mut ctx, user);
    render(tera, "payments/list.html", &ctx)
}

/// Render the create-payment form with an optional error message.
async fn render_form(
    tera: &Tera,
    user: &Authenticated,
    db: &DatabaseConnection,
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let parties = Party::list_active(db).await.unwrap_or_default();
    let accounts = list_accounts(db).await.unwrap_or_default();
    let mut ctx = Context::new();
    ctx.insert("parties", &parties);
    ctx.insert("accounts", &accounts);
    ctx.insert("message", message);
    ctx.insert("message_kind", message_kind);
    insert_nav_context(&mut ctx, user);
    render(tera, "payments/form.html", &ctx)
}

/// List payments with optional filters.
#[get("/payments")]
pub async fn list_payments(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<PaymentFilter>,
) -> HttpResponse {
    render_payments_list(tera.get_ref(), &user, db.get_ref(), &query, "", "").await
}

/// Show the create payment form.
#[get("/payments/new")]
pub async fn new_payment(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
) -> HttpResponse {
    render_form(tera.get_ref(), &user, db.get_ref(), "", "").await
}

/// Create a new payment.
#[post("/payments")]
pub async fn create_payment(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    form: web::Form<PaymentForm>,
) -> HttpResponse {
    let payment_direction = form.payment_direction.trim();
    let amount_str = form.amount.trim();
    let payment_date_str = form.payment_date.trim();
    let from_account_id_str = form.from_account_id.trim();
    let to_account_id_str = form.to_account_id.trim();
    let party_id_str = form.party_id.as_deref().and_then(non_empty);
    let remarks = form
        .remarks
        .as_deref()
        .and_then(non_empty)
        .map(str::to_string);

    let mut errors: Vec<&str> = Vec::new();

    let direction = match PaymentDirection::parse(payment_direction) {
        Some(d) => d,
        None => {
            errors.push("Direction must be IN or OUT.");
            PaymentDirection::In
        }
    };

    let amount = match amount_str.parse::<Decimal>() {
        Ok(a) if a > Decimal::ZERO => a,
        _ => {
            errors.push("Amount must be a positive number.");
            Decimal::ZERO
        }
    };

    let payment_date = match payment_date_str.parse::<NaiveDate>() {
        Ok(d) => d,
        Err(_) => {
            errors.push("Payment date is invalid.");
            chrono::Utc::now().naive_utc().date()
        }
    };

    let from_account_id = match from_account_id_str.parse::<i32>() {
        Ok(id) => id,
        Err(_) => {
            errors.push("Invalid from account.");
            0
        }
    };

    let to_account_id = match to_account_id_str.parse::<i32>() {
        Ok(id) => id,
        Err(_) => {
            errors.push("Invalid to account.");
            0
        }
    };

    let party_id = party_id_str.and_then(|s| s.parse::<i32>().ok());

    if !errors.is_empty() {
        return render_form(
            tera.get_ref(),
            &user,
            db.get_ref(),
            &errors.join(" "),
            "error",
        )
        .await;
    }

    // Use a single transaction to create the payment and its linked journal entry for atomicity.
    let result: Result<Payment, PaymentCreateError> = db
        .transaction(|txn| {
            let remarks = remarks.clone();
            let user_id = user.id;
            Box::pin(async move {
                Payment::create(
                    txn,
                    Payment {
                        id: 0,
                        invoice_id: None,
                        party_id,
                        created_by_user_id: user_id,
                        payment_direction: direction,
                        amount,
                        payment_date,
                        remarks,
                        created_at: chrono::NaiveDateTime::default(),
                    },
                    from_account_id,
                    to_account_id,
                )
                .await
            })
        })
        .await
        .map_err(|e| match e {
            sea_orm::TransactionError::Connection(db_err) => PaymentCreateError::Database(db_err),
            sea_orm::TransactionError::Transaction(p) => p,
        });

    match result {
        Ok(_) => {
            render_payments_list(
                tera.get_ref(),
                &user,
                db.get_ref(),
                &web::Query(PaymentFilter::default()),
                "Payment recorded.",
                "success",
            )
            .await
        }
        Err(PaymentCreateError::SameAccount) => {
            render_form(
                tera.get_ref(),
                &user,
                db.get_ref(),
                "From and to accounts must differ.",
                "error",
            )
            .await
        }
        Err(e) => {
            eprintln!("Failed to record payment: {e}");
            render_form(
                tera.get_ref(),
                &user,
                db.get_ref(),
                "Failed to record payment.",
                "error",
            )
            .await
        }
    }
}

/// Show a single payment.
#[get("/payments/{id}")]
pub async fn show_payment(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let detail = match Payment::find_with_linked(db.get_ref(), id).await {
        Ok(Some(d)) => d,
        Ok(None) => return HttpResponse::NotFound().body("Payment not found."),
        Err(e) => {
            eprintln!("Failed to find payment {id}: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut ctx = Context::new();
    ctx.insert("payment", &detail);
    ctx.insert("message", "");
    ctx.insert("message_kind", "");
    insert_nav_context(&mut ctx, &user);
    render(tera.get_ref(), "payments/show.html", &ctx)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_payments)
        .service(new_payment)
        .service(create_payment)
        .service(show_payment);
}
