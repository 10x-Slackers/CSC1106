//! Home page route.
//!
//! Authors: commit2main
use actix_web::http::StatusCode;
use actix_web::middleware::ErrorHandlers;
use actix_web::{Responder, web};
use sea_orm::DatabaseConnection;
use tera::Context;

use crate::middleware::auth::{Authenticated, redirect_unauthorized};
use crate::models::claim::ClaimStats;
use crate::models::error::AppError;
use crate::models::invoice::InvoiceStats;
use crate::models::party::PartyStats;
use crate::models::user::UserStats;
use crate::routes::utils::{insert_nav_context, render};

/// Render the home page for authenticated users.
pub async fn home(
    user: Authenticated,
    tera: web::Data<tera::Tera>,
    db: web::Data<DatabaseConnection>,
) -> impl Responder {
    let mut context = Context::new();
    insert_nav_context(&mut context, &user);

    let db = db.get_ref();
    let invoice_fut = InvoiceStats::compute(db);
    let claim_fut = async { ClaimStats::compute(db).await.map_err(AppError::from) };
    let party_fut = PartyStats::compute(db);
    let user_fut = UserStats::compute(db);

    if let Ok((invoice_stats, claim_stats, party_stats, user_stats)) =
        futures::try_join!(invoice_fut, claim_fut, party_fut, user_fut)
    {
        context.insert("invoice_stats", &invoice_stats);
        context.insert("claim_stats", &claim_stats);
        context.insert("party_stats", &party_stats);
        context.insert("user_stats", &user_stats);
    } else {
        eprintln!("Failed to compute dashboard stats");
    }

    render(&tera, "home/index.html", &context)
}

/// Configure the home route.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/")
            .wrap(ErrorHandlers::new().handler(StatusCode::UNAUTHORIZED, redirect_unauthorized))
            .route(web::get().to(home)),
    );
}
