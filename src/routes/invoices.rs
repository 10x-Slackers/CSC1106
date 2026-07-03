use actix_web::{HttpResponse, get, post, web};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::TransactionTrait;
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsForm;
use tera::{Context, Tera};

use crate::entity::invoice::InvoiceStatus;
use crate::entity::invoice_line_item::GstRate;
use crate::entity::party::PartyStatus;
use crate::entity::party::PartyType;
use crate::entity::payment::PaymentDirection;
use crate::entity::user::Role;
use crate::middleware::auth::Authenticated;
use crate::middleware::permissions::{Finance, Require};
use crate::models::account::find_cash_and_ar;
use crate::models::error::InvoiceError;
use crate::models::error::PaymentCreateError;
use crate::models::invoice::{
    Invoice, InvoiceLineItem, LineItemInput, grand_total, gst_total, subtotal,
};
use crate::models::party::Party;
use crate::models::payment::Payment;
use crate::models::util::non_empty;
use crate::pdf;
use crate::routes::utils::{
    Pagination, base_query_string, find_or_404, insert_nav_context, parse_field, parse_page,
    pdf_response, render,
};

#[derive(Deserialize, Debug)]
pub struct InvoiceLineItemForm {
    description: String,
    quantity: String,
    unit_price: String,
    gst_rate: String,
}

#[derive(Deserialize, Debug)]
pub struct InvoiceForm {
    party_id: String,
    issue_date: String,
    due_date: String,
    items: Vec<InvoiceLineItemForm>,
}

#[derive(Deserialize)]
struct InvoicePaymentForm {
    amount: String,
    payment_date: String,
    remarks: Option<String>,
}

#[derive(Deserialize, Serialize, Default)]
pub struct InvoiceFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    q: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    party_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    from_issue: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    to_issue: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    from_due: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    to_due: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    page: Option<String>,
}

impl InvoiceFilter {
    fn parsed_party_id(&self) -> Option<i32> {
        self.party_id
            .as_deref()
            .and_then(non_empty)
            .and_then(|s| s.parse().ok())
    }

    fn parsed_date(field: &Option<String>) -> Option<NaiveDate> {
        field
            .as_deref()
            .and_then(non_empty)
            .and_then(|s| s.parse().ok())
    }

    fn parsed_from_issue(&self) -> Option<NaiveDate> {
        Self::parsed_date(&self.from_issue)
    }

    fn parsed_to_issue(&self) -> Option<NaiveDate> {
        Self::parsed_date(&self.to_issue)
    }

    fn parsed_from_due(&self) -> Option<NaiveDate> {
        Self::parsed_date(&self.from_due)
    }

    fn parsed_to_due(&self) -> Option<NaiveDate> {
        Self::parsed_date(&self.to_due)
    }
}

/// Parse submitted line items, skipping rows with a blank description.
fn parse_line_items(form_items: &[InvoiceLineItemForm]) -> (Vec<LineItemInput>, Vec<String>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for (i, item) in form_items.iter().enumerate() {
        let desc = item.description.trim();
        if desc.is_empty() {
            continue;
        }
        let quantity: i32 = match item.quantity.trim().parse() {
            Ok(q) if q > 0 => q,
            Ok(_) => {
                errors.push(format!("Item {}: quantity must be positive.", i + 1));
                1
            }
            Err(_) => {
                errors.push(format!("Item {}: quantity must be a whole number.", i + 1));
                1
            }
        };
        let unit_price: Decimal = match item.unit_price.trim().parse() {
            Ok(p) if p >= Decimal::ZERO => p,
            Ok(_) => {
                errors.push(format!("Item {}: unit price must be non-negative.", i + 1));
                Decimal::ZERO
            }
            Err(_) => {
                errors.push(format!(
                    "Item {}: unit price must be a valid number.",
                    i + 1
                ));
                Decimal::ZERO
            }
        };
        let gst_rate = match GstRate::parse(&item.gst_rate) {
            Some(r) => r,
            None => {
                errors.push(format!(
                    "Item {}: GST rate must be None or Standard.",
                    i + 1
                ));
                GstRate::None
            }
        };
        parsed.push(LineItemInput {
            description: desc.into(),
            quantity,
            unit_price,
            gst_rate,
        });
    }

    (parsed, errors)
}

fn check_ownership(user: &Authenticated, invoice: &Invoice) -> Result<(), HttpResponse> {
    if user.role == Role::Staff && invoice.created_by_user_id != user.id {
        Err(HttpResponse::Forbidden().body("You do not have permission to access this invoice."))
    } else {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn render_invoices_list(
    tera: &Tera,
    user: &Authenticated,
    invoices: &[Invoice],
    parties: &[Party],
    filter: &InvoiceFilter,
    message: &str,
    message_kind: &str,
    pagination: &Pagination,
    base_query: &str,
) -> HttpResponse {
    let mut context = Context::new();
    context.insert("invoices", invoices);
    context.insert("parties", parties);
    context.insert("statuses", &InvoiceStatus::labels());
    context.insert("q", &filter.q);
    context.insert("status", &filter.status);
    context.insert("party_id", &filter.party_id);
    context.insert("from_issue", &filter.from_issue);
    context.insert("to_issue", &filter.to_issue);
    context.insert("from_due", &filter.from_due);
    context.insert("to_due", &filter.to_due);
    context.insert("message", message);
    context.insert("message_kind", message_kind);
    context.insert("pagination", pagination);
    context.insert("pagination_base_query", base_query);
    insert_nav_context(&mut context, user);
    render(tera, "invoices/list.html", &context)
}

async fn reload_invoices_list(
    tera: &Tera,
    user: &Authenticated,
    db: &DatabaseConnection,
    verb: &str,
) -> HttpResponse {
    reload_invoices_list_with(
        tera,
        user,
        db,
        &InvoiceFilter::default(),
        &format!("Invoice {verb}."),
        "success",
    )
    .await
}

async fn reload_invoices_list_with(
    tera: &Tera,
    user: &Authenticated,
    db: &DatabaseConnection,
    filter: &InvoiceFilter,
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let viewer_role = &user.role;
    let viewer_user_id = user.id;
    let base_query = base_query_string(filter);
    match Invoice::list(
        db,
        filter.q.as_deref(),
        filter.status.as_deref().and_then(InvoiceStatus::parse),
        filter.parsed_party_id(),
        filter.parsed_from_issue(),
        filter.parsed_to_issue(),
        filter.parsed_from_due(),
        filter.parsed_to_due(),
        viewer_user_id,
        viewer_role,
        1,
    )
    .await
    {
        Ok((invoices, total_pages, current_page)) => {
            let pagination = Pagination {
                current: current_page,
                total_pages,
            };
            let parties = Party::list(db, Some(PartyType::Customer), None)
                .await
                .unwrap_or_default();
            render_invoices_list(
                tera,
                user,
                &invoices,
                &parties,
                filter,
                message,
                message_kind,
                &pagination,
                &base_query,
            )
        }
        Err(e) => {
            eprintln!("Failed to list invoices: {e}");
            let pagination = Pagination {
                current: 1,
                total_pages: 1,
            };
            let parties = Party::list(db, Some(PartyType::Customer), None)
                .await
                .unwrap_or_default();
            render_invoices_list(
                tera,
                user,
                &[],
                &parties,
                filter,
                "Failed to load invoices.",
                "error",
                &pagination,
                &base_query,
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_invoice_form(
    tera: &Tera,
    user: &Authenticated,
    mode: &str,
    existing_invoice: Option<&Invoice>,
    line_items: Option<&[InvoiceLineItem]>,
    parties: &[Party],
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let mut context = Context::new();
    context.insert("mode", mode);
    context.insert("invoice", &existing_invoice);
    context.insert("line_items", &line_items);
    context.insert("parties", parties);
    context.insert("gst_rates", &GstRate::labels());
    context.insert("entity_label", "Invoice");
    context.insert("base_path", "/invoices");
    if let Some(inv) = existing_invoice {
        context.insert("id", &inv.id);
    }
    context.insert("message", message);
    context.insert("message_kind", message_kind);
    insert_nav_context(&mut context, user);
    render(tera, "invoices/form.html", &context)
}

#[allow(clippy::too_many_arguments)]
fn render_invoice_show(
    tera: &Tera,
    user: &Authenticated,
    invoice: &Invoice,
    line_items: &[InvoiceLineItem],
    party: Option<&Party>,
    payments: &[Payment],
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let mut context = Context::new();
    context.insert("invoice", invoice);
    context.insert("line_items", line_items);
    context.insert("party", &party);
    context.insert("payments", payments);
    context.insert("subtotal", &subtotal(line_items));
    context.insert("gst_total", &gst_total(line_items));
    context.insert("grand_total", &grand_total(line_items));
    context.insert("message", message);
    context.insert("message_kind", message_kind);
    insert_nav_context(&mut context, user);
    render(tera, "invoices/show.html", &context)
}

/// Render the pay-invoice form with an optional error/success message.
fn render_invoice_pay_form(
    tera: &Tera,
    user: &Authenticated,
    invoice: &Invoice,
    party: Option<&Party>,
    outstanding: Decimal,
    message: &str,
    message_kind: &str,
) -> HttpResponse {
    let mut context = Context::new();
    context.insert("invoice", invoice);
    context.insert("party", &party);
    context.insert("outstanding", &outstanding);
    context.insert("today", &chrono::Utc::now().naive_utc().date());
    context.insert("message", message);
    context.insert("message_kind", message_kind);
    insert_nav_context(&mut context, user);
    render(tera, "invoices/pay.html", &context)
}

/// List invoices with optional filters.
#[get("/invoices")]
pub async fn list_invoices(
    user: Authenticated,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<InvoiceFilter>,
) -> HttpResponse {
    let filter = query.into_inner();
    let q = filter.q.as_deref();
    let status = filter.status.as_deref().and_then(InvoiceStatus::parse);
    let viewer_user_id = user.id;
    let viewer_role = &user.role;
    let page = parse_page(filter.page.as_deref());
    let base_query = base_query_string(&filter);

    match Invoice::list(
        db.get_ref(),
        q,
        status,
        filter.parsed_party_id(),
        filter.parsed_from_issue(),
        filter.parsed_to_issue(),
        filter.parsed_from_due(),
        filter.parsed_to_due(),
        viewer_user_id,
        viewer_role,
        page,
    )
    .await
    {
        Ok((invoices, total_pages, current_page)) => {
            let pagination = Pagination {
                current: current_page,
                total_pages,
            };
            let parties = Party::list(db.get_ref(), Some(PartyType::Customer), None)
                .await
                .unwrap_or_default();
            render_invoices_list(
                tera.get_ref(),
                &user,
                &invoices,
                &parties,
                &filter,
                "",
                "",
                &pagination,
                &base_query,
            )
        }
        Err(e) => {
            eprintln!("Failed to list invoices: {e}");
            let pagination = Pagination {
                current: 1,
                total_pages: 1,
            };
            let parties = Party::list(db.get_ref(), Some(PartyType::Customer), None)
                .await
                .unwrap_or_default();
            render_invoices_list(
                tera.get_ref(),
                &user,
                &[],
                &parties,
                &filter,
                "Failed to load invoices.",
                "error",
                &pagination,
                &base_query,
            )
        }
    }
}

/// Show the create-invoice form.
#[get("/invoices/new")]
pub async fn new_invoice(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
) -> HttpResponse {
    let parties = Party::list(
        db.get_ref(),
        Some(PartyType::Customer),
        Some(PartyStatus::Active),
    )
    .await
    .unwrap_or_default();
    render_invoice_form(tera.get_ref(), &user, "new", None, None, &parties, "", "")
}

/// Create a new invoice.
#[post("/invoices")]
pub async fn create_invoice(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    form: QsForm<InvoiceForm>,
) -> HttpResponse {
    let mut errors: Vec<String> = Vec::new();

    let party_id: i32 = parse_field(&form.party_id, &mut errors, "Party is required.", 0);
    let issue_date: NaiveDate = parse_field(
        &form.issue_date,
        &mut errors,
        "Issue date is invalid.",
        NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
    );
    let due_date: NaiveDate = parse_field(
        &form.due_date,
        &mut errors,
        "Due date is invalid.",
        NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
    );

    if due_date < issue_date {
        errors.push("Due date must be on or after issue date.".into());
    }

    let (parsed_items, item_errors) = parse_line_items(&form.items);
    errors.extend(item_errors);
    if parsed_items.is_empty() {
        errors.push("At least one line item is required.".into());
    }

    if !errors.is_empty() {
        let parties = Party::list(
            db.get_ref(),
            Some(PartyType::Customer),
            Some(PartyStatus::Active),
        )
        .await
        .unwrap_or_default();
        return render_invoice_form(
            tera.get_ref(),
            &user,
            "new",
            None,
            None,
            &parties,
            &errors.join(" "),
            "error",
        );
    }

    match Invoice::create(
        db.get_ref(),
        party_id,
        user.id,
        issue_date,
        due_date,
        parsed_items,
    )
    .await
    {
        Ok(_) => reload_invoices_list(tera.get_ref(), &user, db.get_ref(), "created").await,
        Err(InvoiceError::Duplicate) => {
            let parties = Party::list(
                db.get_ref(),
                Some(PartyType::Customer),
                Some(PartyStatus::Active),
            )
            .await
            .unwrap_or_default();
            render_invoice_form(
                tera.get_ref(),
                &user,
                "new",
                None,
                None,
                &parties,
                "Invoice number collision. Please try again.",
                "error",
            )
        }
        Err(InvoiceError::ValidationError(msg)) => {
            let parties = Party::list(
                db.get_ref(),
                Some(PartyType::Customer),
                Some(PartyStatus::Active),
            )
            .await
            .unwrap_or_default();
            render_invoice_form(
                tera.get_ref(),
                &user,
                "new",
                None,
                None,
                &parties,
                &msg,
                "error",
            )
        }
        Err(e) => {
            eprintln!("Failed to create invoice: {e}");
            let parties = Party::list(
                db.get_ref(),
                Some(PartyType::Customer),
                Some(PartyStatus::Active),
            )
            .await
            .unwrap_or_default();
            render_invoice_form(
                tera.get_ref(),
                &user,
                "new",
                None,
                None,
                &parties,
                "Failed to create invoice due to an internal error.",
                "error",
            )
        }
    }
}

/// Show invoice detail.
#[get("/invoices/{id}")]
pub async fn show_invoice(
    user: Authenticated,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let (invoice, line_items) =
        match find_or_404(Invoice::find_by_id(db.get_ref(), id), id, "Invoice").await {
            Ok(r) => r,
            Err(resp) => return resp,
        };

    if let Err(resp) = check_ownership(&user, &invoice) {
        return resp;
    }

    let party = Party::find_by_id(db.get_ref(), invoice.party_id)
        .await
        .unwrap_or(None);

    let payments = Payment::list_for_invoice(db.get_ref(), invoice.id)
        .await
        .unwrap_or_default();

    render_invoice_show(
        tera.get_ref(),
        &user,
        &invoice,
        &line_items,
        party.as_ref(),
        &payments,
        "",
        "",
    )
}

/// Download an invoice as a PDF document.
#[get("/invoices/{id}/pdf")]
pub async fn invoice_pdf(
    user: Authenticated,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let (invoice, line_items) =
        match find_or_404(Invoice::find_by_id(db.get_ref(), id), id, "Invoice").await {
            Ok(r) => r,
            Err(resp) => return resp,
        };

    if let Err(resp) = check_ownership(&user, &invoice) {
        return resp;
    }

    let party = Party::find_by_id(db.get_ref(), invoice.party_id)
        .await
        .unwrap_or(None);

    let filename = format!("{}.pdf", invoice.invoice_no);
    let result =
        web::block(move || pdf::render_invoice(&invoice, &line_items, party.as_ref())).await;

    pdf_response(result, &filename, &format!("Invoice {id}"))
}

/// Show the simplified payment form for an invoice.
#[get("/invoices/{id}/pay")]
pub async fn pay_invoice_form(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let (invoice, line_items) =
        match find_or_404(Invoice::find_by_id(db.get_ref(), id), id, "Invoice").await {
            Ok(r) => r,
            Err(resp) => return resp,
        };

    if let Err(resp) = check_ownership(&user, &invoice) {
        return resp;
    }

    let party = Party::find_by_id(db.get_ref(), invoice.party_id)
        .await
        .unwrap_or(None);

    // Only Sent or PartiallyPaid invoices can receive payments.
    match invoice.status {
        InvoiceStatus::Sent | InvoiceStatus::PartiallyPaid => {
            let total = grand_total(&line_items);
            let paid = Payment::total_for_invoice(db.get_ref(), invoice.id)
                .await
                .unwrap_or(Decimal::ZERO);
            let outstanding = total - paid;
            render_invoice_pay_form(
                tera.get_ref(),
                &user,
                &invoice,
                party.as_ref(),
                outstanding,
                "",
                "",
            )
        }
        InvoiceStatus::Draft => render_invoice_show(
            tera.get_ref(),
            &user,
            &invoice,
            &line_items,
            party.as_ref(),
            &[],
            "Send the invoice before recording a payment.",
            "error",
        ),
        _ => render_invoice_show(
            tera.get_ref(),
            &user,
            &invoice,
            &line_items,
            party.as_ref(),
            &[],
            "This invoice cannot be paid.",
            "error",
        ),
    }
}

/// Record a payment against an invoice using the simplified form.
#[post("/invoices/{id}/pay")]
pub async fn pay_invoice(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    form: web::Form<InvoicePaymentForm>,
) -> HttpResponse {
    let id = path.into_inner();

    let (invoice, line_items) =
        match find_or_404(Invoice::find_by_id(db.get_ref(), id), id, "Invoice").await {
            Ok(r) => r,
            Err(resp) => return resp,
        };

    if let Err(resp) = check_ownership(&user, &invoice) {
        return resp;
    }

    let party = Party::find_by_id(db.get_ref(), invoice.party_id)
        .await
        .unwrap_or(None);

    // Re-check status (guards against stale form / double submit).
    if !matches!(
        invoice.status,
        InvoiceStatus::Sent | InvoiceStatus::PartiallyPaid
    ) {
        return render_invoice_show(
            tera.get_ref(),
            &user,
            &invoice,
            &line_items,
            party.as_ref(),
            &[],
            "This invoice cannot be paid.",
            "error",
        );
    }

    let total = grand_total(&line_items);
    let paid = Payment::total_for_invoice(db.get_ref(), invoice.id)
        .await
        .unwrap_or(Decimal::ZERO);
    let outstanding = total - paid;

    let amount_str = form.amount.trim();
    let payment_date_str = form.payment_date.trim();
    let remarks = form
        .remarks
        .as_deref()
        .and_then(non_empty)
        .map(str::to_string);

    let mut errors: Vec<String> = Vec::new();

    let amount = match amount_str.parse::<Decimal>() {
        Ok(a) if a > Decimal::ZERO => a,
        _ => {
            errors.push("Amount must be a positive number.".into());
            Decimal::ZERO
        }
    };

    if errors.is_empty() && amount > outstanding {
        errors.push(format!(
            "Payment exceeds the outstanding balance of {}.",
            outstanding
        ));
    }

    let payment_date = parse_field(
        payment_date_str,
        &mut errors,
        "Payment date is invalid.",
        chrono::Utc::now().naive_utc().date(),
    );

    if !errors.is_empty() {
        return render_invoice_pay_form(
            tera.get_ref(),
            &user,
            &invoice,
            party.as_ref(),
            outstanding,
            &errors.join(" "),
            "error",
        );
    }

    // Look up Cash (debit) and Accounts Receivable (credit) accounts.
    let (cash, ar) = match find_cash_and_ar(db.get_ref()).await {
        Ok(pair) => pair,
        Err(_) => {
            return render_invoice_pay_form(
                tera.get_ref(),
                &user,
                &invoice,
                party.as_ref(),
                outstanding,
                "Cash or Accounts Receivable account is not configured.",
                "error",
            );
        }
    };

    let user_id = user.id;
    let result: Result<Payment, PaymentCreateError> = db
        .transaction(|txn| {
            let remarks = remarks.clone();
            Box::pin(async move {
                Payment::create(
                    txn,
                    Payment {
                        id: 0,
                        invoice_id: Some(invoice.id),
                        party_id: Some(invoice.party_id),
                        created_by_user_id: user_id,
                        payment_direction: PaymentDirection::In,
                        amount,
                        payment_date,
                        remarks,
                        created_at: chrono::NaiveDateTime::default(),
                    },
                    ar.id,
                    cash.id,
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
            let payments = Payment::list_for_invoice(db.get_ref(), invoice.id)
                .await
                .unwrap_or_default();
            render_invoice_show(
                tera.get_ref(),
                &user,
                &invoice,
                &line_items,
                party.as_ref(),
                &payments,
                "Payment recorded.",
                "success",
            )
        }
        Err(PaymentCreateError::SameAccount) => render_invoice_pay_form(
            tera.get_ref(),
            &user,
            &invoice,
            party.as_ref(),
            outstanding,
            "Cash and Accounts Receivable must differ.",
            "error",
        ),
        Err(e) => {
            eprintln!("Failed to record payment for invoice {id}: {e}");
            render_invoice_pay_form(
                tera.get_ref(),
                &user,
                &invoice,
                party.as_ref(),
                outstanding,
                "Failed to record payment.",
                "error",
            )
        }
    }
}

/// Show the edit-invoice form.
#[get("/invoices/{id}/edit")]
pub async fn edit_invoice(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let (invoice, line_items) =
        match find_or_404(Invoice::find_by_id(db.get_ref(), id), id, "Invoice").await {
            Ok(r) => r,
            Err(resp) => return resp,
        };

    if let Err(resp) = check_ownership(&user, &invoice) {
        return resp;
    }

    let parties = Party::list(
        db.get_ref(),
        Some(PartyType::Customer),
        Some(PartyStatus::Active),
    )
    .await
    .unwrap_or_default();
    render_invoice_form(
        tera.get_ref(),
        &user,
        "edit",
        Some(&invoice),
        Some(&line_items),
        &parties,
        "",
        "",
    )
}

/// Update an existing invoice.
#[post("/invoices/{id}")]
pub async fn update_invoice(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    form: QsForm<InvoiceForm>,
) -> HttpResponse {
    let id = path.into_inner();

    let (existing_invoice, existing_items) =
        match find_or_404(Invoice::find_by_id(db.get_ref(), id), id, "Invoice").await {
            Ok(r) => r,
            Err(resp) => return resp,
        };

    if let Err(resp) = check_ownership(&user, &existing_invoice) {
        return resp;
    }

    let mut errors: Vec<String> = Vec::new();
    let issue_date: NaiveDate = parse_field(
        &form.issue_date,
        &mut errors,
        "Issue date is invalid.",
        NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
    );
    let due_date: NaiveDate = parse_field(
        &form.due_date,
        &mut errors,
        "Due date is invalid.",
        NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
    );

    if due_date < issue_date {
        errors.push("Due date must be on or after issue date.".into());
    }

    let (parsed_items, item_errors) = parse_line_items(&form.items);
    errors.extend(item_errors);
    if parsed_items.is_empty() {
        errors.push("At least one line item is required.".into());
    }

    if !errors.is_empty() {
        let parties = Party::list(
            db.get_ref(),
            Some(PartyType::Customer),
            Some(PartyStatus::Active),
        )
        .await
        .unwrap_or_default();
        return render_invoice_form(
            tera.get_ref(),
            &user,
            "edit",
            Some(&existing_invoice),
            Some(&existing_items),
            &parties,
            &errors.join(" "),
            "error",
        );
    }

    match existing_invoice
        .update(db.get_ref(), issue_date, due_date, parsed_items)
        .await
    {
        Ok(_) => reload_invoices_list(tera.get_ref(), &user, db.get_ref(), "updated").await,
        Err(InvoiceError::ValidationError(msg)) => {
            let parties = Party::list(
                db.get_ref(),
                Some(PartyType::Customer),
                Some(PartyStatus::Active),
            )
            .await
            .unwrap_or_default();
            render_invoice_form(
                tera.get_ref(),
                &user,
                "edit",
                Some(&existing_invoice),
                Some(&existing_items),
                &parties,
                &msg,
                "error",
            )
        }
        Err(e) => {
            eprintln!("Failed to update invoice {id}: {e}");
            let parties = Party::list(
                db.get_ref(),
                Some(PartyType::Customer),
                Some(PartyStatus::Active),
            )
            .await
            .unwrap_or_default();
            render_invoice_form(
                tera.get_ref(),
                &user,
                "edit",
                Some(&existing_invoice),
                Some(&existing_items),
                &parties,
                "Failed to update invoice due to an internal error.",
                "error",
            )
        }
    }
}

/// Send (post) an invoice.
#[post("/invoices/{id}/send")]
pub async fn send_invoice(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let (invoice, _) = match find_or_404(Invoice::find_by_id(db.get_ref(), id), id, "Invoice").await
    {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    if let Err(resp) = check_ownership(&user, &invoice) {
        return resp;
    }

    match invoice.send(db.get_ref(), user.id).await {
        Ok(_) => reload_invoices_list(tera.get_ref(), &user, db.get_ref(), "sent").await,
        Err(e) => {
            eprintln!("Failed to send invoice {id}: {e}");
            reload_invoices_list_with(
                tera.get_ref(),
                &user,
                db.get_ref(),
                &InvoiceFilter::default(),
                "Failed to send invoice due to an internal error.",
                "error",
            )
            .await
        }
    }
}

/// Void an invoice.
#[post("/invoices/{id}/void")]
pub async fn void_invoice(
    user: Require<Finance>,
    tera: web::Data<Tera>,
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> HttpResponse {
    let id = path.into_inner();

    let (invoice, _) = match find_or_404(Invoice::find_by_id(db.get_ref(), id), id, "Invoice").await
    {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    if let Err(resp) = check_ownership(&user, &invoice) {
        return resp;
    }

    match invoice.void(db.get_ref(), user.id).await {
        Ok(_) => reload_invoices_list(tera.get_ref(), &user, db.get_ref(), "voided").await,
        Err(e) => {
            eprintln!("Failed to void invoice {id}: {e}");
            reload_invoices_list_with(
                tera.get_ref(),
                &user,
                db.get_ref(),
                &InvoiceFilter::default(),
                "Failed to void invoice due to an internal error.",
                "error",
            )
            .await
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_invoices)
        .service(new_invoice)
        .service(create_invoice)
        .service(show_invoice)
        .service(invoice_pdf)
        .service(edit_invoice)
        .service(update_invoice)
        .service(send_invoice)
        .service(void_invoice)
        .service(pay_invoice_form)
        .service(pay_invoice);
}
