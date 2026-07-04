//! Routes for viewing the four financial reports as HTML pages.
//! and to download each report as a PDF.
//!
//! Authors: Felicia

use actix_web::{HttpResponse, Responder, get, web};
use chrono::{Datelike, NaiveDate};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Deserializer};
use tera::Context;

use crate::middleware::permissions::{Finance, Require};
use crate::models::report::{AuditStatement, BalanceSheetReport, GstReport, IncomeStatementReport};
use crate::pdf;
use crate::routes::utils::{insert_nav_context, pdf_response, render};

/// Deserialize an optional date, treating empty strings as None.
fn opt_date<'de, D: Deserializer<'de>>(d: D) -> Result<Option<NaiveDate>, D::Error> {
    let opt: Option<String> = Option::deserialize(d)?;
    opt.filter(|s| !s.is_empty())
        .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(serde::de::Error::custom))
        .transpose()
}

#[derive(Deserialize, Default)]
pub struct DateRangeFilter {
    #[serde(default, deserialize_with = "opt_date")]
    pub from: Option<NaiveDate>,
    #[serde(default, deserialize_with = "opt_date")]
    pub to: Option<NaiveDate>,
}

impl DateRangeFilter {
    /// Returns an error message if `from` is later than `to`.
    fn validate(&self) -> Result<(), &'static str> {
        if let (Some(f), Some(t)) = (self.from, self.to)
            && f > t
        {
            return Err("'from' date must not be later than 'to' date");
        }
        Ok(())
    }
}

#[derive(Deserialize, Default)]
pub struct AsOfFilter {
    #[serde(default, deserialize_with = "opt_date")]
    pub as_of: Option<NaiveDate>,
}

#[derive(Deserialize, Default)]
pub struct YearFilter {
    #[serde(default)]
    pub year: Option<i32>,
}

/// Show links to the four reports (GST, income statement, balance sheet, audit).
#[get("/reports")]
pub async fn reports_index(user: Require<Finance>, tera: web::Data<tera::Tera>) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);
    render(&tera, "reports/index.html", &context)
}

/// Show the GST summary report for an optional date range.
#[get("/reports/gst")]
pub async fn gst_report(
    user: Require<Finance>,
    tera: web::Data<tera::Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<DateRangeFilter>,
) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);

    if let Err(msg) = query.validate() {
        return HttpResponse::BadRequest().body(msg);
    }

    context.insert("from", &query.from);
    context.insert("to", &query.to);

    match GstReport::compute(&db, query.from, query.to).await {
        Ok(report) => {
            context.insert("report", &report);
        }
        Err(e) => {
            eprintln!("GST report error: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    render(&tera, "reports/gst.html", &context)
}

/// Show the income statement for an optional date range.
#[get("/reports/income-statement")]
pub async fn income_statement(
    user: Require<Finance>,
    tera: web::Data<tera::Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<DateRangeFilter>,
) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);

    if let Err(msg) = query.validate() {
        return HttpResponse::BadRequest().body(msg);
    }

    context.insert("from", &query.from);
    context.insert("to", &query.to);

    let from_dt = query.from.map(|d| d.and_hms_opt(0, 0, 0).unwrap());
    let to_dt = query.to.map(|d| d.and_hms_opt(23, 59, 59).unwrap());

    match IncomeStatementReport::compute(&db, from_dt, to_dt).await {
        Ok(report) => {
            context.insert("report", &report);
        }
        Err(e) => {
            eprintln!("Income statement error: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    render(&tera, "reports/income_statement.html", &context)
}

/// Download the income statement as a PDF.
#[get("/reports/income-statement/pdf")]
pub async fn income_statement_pdf(
    _user: Require<Finance>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<DateRangeFilter>,
) -> HttpResponse {
    if let Err(msg) = query.validate() {
        return HttpResponse::BadRequest().body(msg);
    }

    let from_dt = query.from.map(|d| d.and_hms_opt(0, 0, 0).unwrap());
    let to_dt = query.to.map(|d| d.and_hms_opt(23, 59, 59).unwrap());

    let report = match IncomeStatementReport::compute(&db, from_dt, to_dt).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Income statement PDF data error: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    pdf_response(
        web::block(move || pdf::render_income_statement(&report)).await,
        "income-statement.pdf",
        "Income statement",
    )
}

/// Download the GST summary as a PDF.
#[get("/reports/gst/pdf")]
pub async fn gst_pdf(
    _user: Require<Finance>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<DateRangeFilter>,
) -> HttpResponse {
    if let Err(msg) = query.validate() {
        return HttpResponse::BadRequest().body(msg);
    }

    let report = match GstReport::compute(&db, query.from, query.to).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("GST PDF data error: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    pdf_response(
        web::block(move || pdf::render_gst(&report)).await,
        "gst-summary.pdf",
        "GST",
    )
}

/// Show the balance sheet as of a given date, or the latest balances if none is given.
#[get("/reports/balance-sheet")]
pub async fn balance_sheet(
    user: Require<Finance>,
    tera: web::Data<tera::Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<AsOfFilter>,
) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);
    context.insert("as_of", &query.as_of);

    let as_of_dt = query.as_of.map(|d| d.and_hms_opt(23, 59, 59).unwrap());

    match BalanceSheetReport::compute(&db, as_of_dt).await {
        Ok(report) => {
            context.insert("report", &report);
        }
        Err(e) => {
            eprintln!("Balance sheet error: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    render(&tera, "reports/balance_sheet.html", &context)
}

/// Download the balance sheet as a PDF.
#[get("/reports/balance-sheet/pdf")]
pub async fn balance_sheet_pdf(
    _user: Require<Finance>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<AsOfFilter>,
) -> HttpResponse {
    let as_of_dt = query.as_of.map(|d| d.and_hms_opt(23, 59, 59).unwrap());

    let report = match BalanceSheetReport::compute(&db, as_of_dt).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Balance sheet PDF data error: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    pdf_response(
        web::block(move || pdf::render_balance_sheet(&report)).await,
        "balance-sheet.pdf",
        "Balance sheet",
    )
}

/// Download the audit statement as a PDF for the given year, defaulting to the current year.
#[get("/reports/audit/pdf")]
pub async fn audit_pdf(
    _user: Require<Finance>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<YearFilter>,
) -> HttpResponse {
    let year = query.year.unwrap_or_else(|| chrono::Utc::now().year());
    if !(1900..=2100).contains(&year) {
        return HttpResponse::BadRequest().body("year must be between 1900 and 2100");
    }

    let report = match AuditStatement::compute(&db, year).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Audit PDF data error: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    pdf_response(
        web::block(move || pdf::render_audit(&report)).await,
        "audit-statement.pdf",
        "Audit",
    )
}

/// Show the audit statement (journal entries) for the given year, defaulting to the current year.
#[get("/reports/audit")]
pub async fn audit_report(
    user: Require<Finance>,
    tera: web::Data<tera::Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<YearFilter>,
) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);

    let year = query.year.unwrap_or_else(|| chrono::Utc::now().year());
    if !(1900..=2100).contains(&year) {
        return HttpResponse::BadRequest().body("year must be between 1900 and 2100");
    }
    context.insert("year", &year);

    match AuditStatement::compute(&db, year).await {
        Ok(report) => {
            context.insert("report", &report);
        }
        Err(e) => {
            eprintln!("Audit report error: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    render(&tera, "reports/audit.html", &context)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(reports_index)
        .service(gst_report)
        .service(gst_pdf)
        .service(income_statement)
        .service(income_statement_pdf)
        .service(balance_sheet)
        .service(balance_sheet_pdf)
        .service(audit_report)
        .service(audit_pdf);
}
