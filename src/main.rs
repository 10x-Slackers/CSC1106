pub mod db;
mod entities;
mod middleware;
mod models;
mod routes;

use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use sea_orm::{DatabaseConnection, EntityTrait, Iterable, JoinType, QuerySelect, RelationTrait};
use tera::Tera;

use crate::db::init_db;

use entities::{customer, invoice, user};

struct AppState {
    pool: DatabaseConnection,
}

#[actix_web::get("/")]
async fn home(data: web::Data<AppState>) -> impl Responder {
    let users = user::Entity::find().all(&data.pool).await.unwrap();
    HttpResponse::Ok().json(users)
}

#[actix_web::get("/invoice")]
async fn get_invoices(data: web::Data<AppState>) -> impl Responder {
    let invoices = invoice::Entity::find()
        .select_only()
        .columns(invoice::Column::iter())
        .column_as(customer::Column::Name, "customer_name")
        .column_as(user::Column::Name, "created_by_name")
        .join(JoinType::LeftJoin, invoice::Relation::Customer.def())
        .join(JoinType::LeftJoin, invoice::Relation::CreatedBy.def())
        .into_model::<invoice::InvoiceRow>()
        .all(&data.pool)
        .await
        .unwrap();
    HttpResponse::Ok().json(invoices)
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
    let pool = init_db().await.unwrap();

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
            .service(get_invoices)
            .service(Files::new("/css", "assets/css"))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
