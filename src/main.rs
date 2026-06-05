mod middleware;
mod models;
mod routes;

use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpServer, web};
use tera::Tera;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "localhost".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    let secret_key = Key::generate();

    let tera = Tera::new("templates/**/*").expect("Failed to initialize Tera templates");

    println!("Server running at http://{host}:{port}");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(tera.clone()))
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false)
                    .build(),
            )
            .configure(routes::configure)
            .service(Files::new("/css", "assets/css"))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
