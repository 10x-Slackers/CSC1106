pub mod auth;
pub mod home;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(home::configure).configure(auth::configure);
}
