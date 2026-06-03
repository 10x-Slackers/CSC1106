mod models;
mod routes;

use actix_identity::IdentityMiddleware;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::cookie::Key;
use actix_web::{get, web, App, HttpMessage, HttpResponse, HttpServer, Responder};
use actix_identity::Identity;
use tera::Tera;

#[get("/")]
async fn home(user: Option<Identity>) -> impl Responder {
    if user.is_some() {
        HttpResponse::Found()
            .append_header(("Location", "/dashboard"))
            .finish()
    } else {
        HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish()
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let secret_key = Key::generate();

    let tera = Tera::new("templates/**/*")
        .expect("Failed to initialize Tera templates");

    println!("Server running at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(tera.clone()))
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false)
                    .build(),
            )
            .service(home)
            .service(routes::auth::show_login)
            .service(routes::auth::process_login)
            .service(routes::auth::logout)
            .service(routes::dashboard::dashboard)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
