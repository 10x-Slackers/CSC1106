//! Main entry point for the Actix-web application.
//!
//! Authors: commit2main

mod db;
mod entity;
mod middleware;
mod models;
mod pdf;
mod routes;
mod seed;

use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::http::StatusCode;
use actix_web::middleware::{ErrorHandlers, from_fn};
use actix_web::{App, HttpServer, web};
use tera::Tera;

use crate::middleware::auth::{UserCache, show_unauthorized_page};
use crate::middleware::permissions::forbidden_page;
use crate::middleware::tera_filters;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "localhost".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://./data.db?mode=rwc".to_string());
    let secret_key = std::env::var("SECRET_KEY")
        .map(|s| Key::derive_from(s.as_bytes()))
        .unwrap_or_else(|_| {
            eprintln!("warning: SECRET_KEY not set; generating a random key — sessions will not persist across restarts");
            Key::generate()
        });

    let db = db::init(&database_url).await;

    if db::has_no_users(&db).await {
        seed::create_admin_interactively(&db).await;
    }

    let mut tera = Tera::new("templates/**/*").expect("Failed to initialize Tera templates");
    tera_filters::register(&mut tera);
    let user_cache = UserCache::new();

    println!("Server running at http://{host}:{port}");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(user_cache.clone()))
            .wrap(from_fn(
                crate::middleware::security_headers::security_headers,
            ))
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false)
                    .cookie_http_only(true)
                    .cookie_same_site(actix_web::cookie::SameSite::Lax)
                    .build(),
            )
            .wrap(IdentityMiddleware::default())
            .wrap(
                ErrorHandlers::new()
                    .handler(StatusCode::UNAUTHORIZED, show_unauthorized_page)
                    .handler(StatusCode::FORBIDDEN, forbidden_page),
            )
            .configure(routes::configure)
            .service(Files::new("/", "assets/"))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
