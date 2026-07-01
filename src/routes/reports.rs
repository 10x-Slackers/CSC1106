use actix_web::{HttpResponse, Responder, get, web};
use chrono::{Datelike, NaiveDate};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Deserializer};
use tera::Context;

use crate::middleware::permissions::{Finance, Require};
use crate::models::report::{AuditStatement, BalanceSheetReport, GstReport, IncomeStatementReport};
use crate::routes::utils::{insert_nav_context, render};

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

#[get("/reports")]
pub async fn reports_index(user: Require<Finance>, tera: web::Data<tera::Tera>) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);
    render(&tera, "reports/index.html", &context)
}

#[get("/reports/gst")]
pub async fn gst_report(
    user: Require<Finance>,
    tera: web::Data<tera::Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<DateRangeFilter>,
) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);
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

#[get("/reports/income-statement")]
pub async fn income_statement(
    user: Require<Finance>,
    tera: web::Data<tera::Tera>,
    db: web::Data<DatabaseConnection>,
    query: web::Query<DateRangeFilter>,
) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);
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
        .service(income_statement)
        .service(balance_sheet)
        .service(audit_report);
}
