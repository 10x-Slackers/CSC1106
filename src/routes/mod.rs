pub mod auth;
pub mod claims;
pub mod home;
pub mod invoices;
pub mod nav;
pub mod parties;
pub mod reports;
pub mod users;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(home::configure)
        .configure(auth::configure)
        .configure(invoices::configure)
        .configure(claims::configure)
        .configure(parties::configure)
        .configure(reports::configure)
        .configure(users::configure);
}
