mod middleware;
mod models;
mod routes;

use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use sqlx::{
    Error, Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tera::Tera;

struct AppState {
    db: Pool<Sqlite>,
}

#[derive(sqlx::FromRow)]
struct User {
    id: i64,
    name: String,
    email: String,
    role: String,
}

#[actix_web::get("/")]
async fn home(data: web::Data<AppState>) -> impl Responder {
    let response = "<h1>Welcome to the Accounting App</h1>";
    let users = sqlx::query_as::<_, User>("SELECT id, name, email, role FROM users")
        .fetch_all(&data.db)
        .await
        .unwrap();
    let mut user_list = String::from("<ul>");
    for user in users {
        user_list.push_str(&format!("<li>{} - {} - {:?}</li>", user.name, user.email, user.role));
    }
    user_list.push_str("</ul>");
    let response = format!("{}{}", response, user_list);
    HttpResponse::Ok().content_type("text/html").body(response)
}

async fn create_db() -> Result<Pool<Sqlite>, Error> {
    let db_url = "accounting.db";
    let db_exist = std::path::Path::new(db_url).exists();

    let options = SqliteConnectOptions::new()
        .filename(db_url)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    if !db_exist {
        println!("Database created at {}", db_url);
        sqlx::migrate!().run(&pool).await?;
    }
    Ok(pool)
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
            .app_data(web::Data::new(AppState { db: pool.clone() }))
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