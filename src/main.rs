mod middleware;
mod models;
mod routes;
pub mod db;

use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use sqlx::{Pool, Sqlite};
use tera::Tera;

use db::{create_db, fetch_users};

struct AppState {
    pool: Pool<Sqlite>,
}

#[actix_web::get("/")]
async fn home(data: web::Data<AppState>) -> impl Responder {
    let response = "<h1>Welcome to the Accounting App</h1>";
    let users = fetch_users(&data.pool).await.unwrap();
    let mut user_list = String::from("<ul>");
    for user in users {
        user_list.push_str(&format!(
            "<li>{} - {} - {:?}</li>",
            user.name, user.email, user.role
        ));
    }
    user_list.push_str("</ul>");
    let response = format!("{}{}", response, user_list);
    HttpResponse::Ok().content_type("text/html").body(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "localhost".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let secret_key = std::env::var("SECRET_KEY")
        .map(|s| Key::derive_from(s.as_bytes()))
        .unwrap_or_else(|_| Key::generate());

    let tera = Tera::new("templates/**/*").expect("Failed to initialize Tera templates");
    let pool = create_db()
        .await
        .expect("Failed to create database connection");

    println!("Server running at http://{host}:{port}");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(AppState { pool: pool.clone() }))
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false) // No HTTPS for the project, not deployed
                    .build(),
            )
            .wrap(IdentityMiddleware::default())
            .configure(routes::configure)
            .service(home)
            .service(Files::new("/css", "assets/css"))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}