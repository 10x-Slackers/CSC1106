use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use askama::Template;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    message: Option<String>,
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

#[get("/")]
async fn home() -> impl Responder {
    HttpResponse::Found()
        .append_header(("Location", "/login"))
        .finish()
}

#[get("/login")]
async fn show_login() -> impl Responder {
    let page = LoginTemplate {
        message: None,
    };

    HttpResponse::Ok()
        .content_type("text/html")
        .body(page.render().unwrap())
}

#[post("/login")]
async fn process_login(form: web::Form<LoginForm>) -> impl Responder {
    if form.username == "admin" && form.password == "admin123" {
        HttpResponse::Ok()
            .content_type("text/html")
            .body("<h1>Login successful</h1><p>Welcome, admin!</p>")
    } else {
        let page = LoginTemplate {
            message: Some("Wrong username or password".to_string()),
        };

        HttpResponse::Ok()
            .content_type("text/html")
            .body(page.render().unwrap())
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server running at http://localhost:8080");

    HttpServer::new(|| {
        App::new()
            .service(home)
            .service(show_login)
            .service(process_login)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
