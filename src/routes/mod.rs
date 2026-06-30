pub mod auth;
pub mod claims;
pub mod home;
pub mod invoices;
pub mod pagination;
pub mod parties;
pub mod payments;
pub mod profile;
pub mod reports;
pub mod users;
pub mod utils;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(home::configure)
        .configure(auth::configure)
        .configure(invoices::configure)
        .configure(claims::configure)
        .configure(parties::configure)
        .configure(payments::configure)
        .configure(reports::configure)
        .configure(users::configure)
        .configure(profile::configure);
}
