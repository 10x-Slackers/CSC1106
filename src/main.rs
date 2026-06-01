use actix_identity::Identity;
use actix_identity::IdentityMiddleware;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::cookie::Key;
use actix_web::{get, post, web, App, HttpMessage, HttpRequest, HttpResponse, HttpServer, Responder};
use askama::Template;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    message: Option<String>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    username: String,
}

// TODO: Replace this with a real database model
struct User {
    username: &'static str,
    password: &'static str,
    display_name: &'static str,
}

// TODO: Replace this with a real database lookup
fn find_user(username: &str) -> Option<&'static User> {
    static USERS: &[User] = &[
        User {
            username: "admin",
            password: "admin123",
            display_name: "Administrator",
        },
        User {
            username: "alice",
            password: "alice123",
            display_name: "Alice (Accountant)",
        },
    ];

    USERS.iter().find(|u| u.username == username)
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

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

#[get("/login")]
async fn show_login(user: Option<Identity>) -> impl Responder {
    if user.is_some() {
        return HttpResponse::Found()
            .append_header(("Location", "/dashboard"))
            .finish();
    }

    render_login(None)
}

#[post("/login")]
async fn process_login(
    req: HttpRequest,
    form: web::Form<LoginForm>,
) -> impl Responder {
    // TODO: Implement proper authentication with hashed passwords and database lookup
    match find_user(&form.username) {
        Some(user) if user.password == form.password => {
            Identity::login(&req.extensions(), user.username.to_string())
                .expect("Failed to create identity");

            HttpResponse::Found()
                .append_header(("Location", "/dashboard"))
                .finish()
        }
        _ => render_login(Some("Wrong username or password.".to_string())),
    }
}

#[get("/dashboard")]
async fn dashboard(user: Option<Identity>) -> impl Responder {
    match user {
        Some(identity) => {
            let username = identity.id().unwrap_or_else(|_| "unknown".to_string());
            let page = DashboardTemplate { username };
            HttpResponse::Ok()
                .content_type("text/html")
                .body(page.render().unwrap())
        }
        None => HttpResponse::Found()
            .append_header(("Location", "/login"))
            .finish(),
    }
}

#[post("/logout")]
async fn logout(user: Option<Identity>) -> impl Responder {
    if let Some(identity) = user {
        identity.logout();
    }
    HttpResponse::Found()
        .append_header(("Location", "/login"))
        .finish()
}

fn render_login(message: Option<String>) -> HttpResponse {
    let page = LoginTemplate { message };
    HttpResponse::Ok()
        .content_type("text/html")
        .body(page.render().unwrap())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let secret_key = Key::generate();

    println!("Server running at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false)
                    .build(),
            )
            .wrap(IdentityMiddleware::default())
            .service(home)
            .service(show_login)
            .service(process_login)
            .service(dashboard)
            .service(logout)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
