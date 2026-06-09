use actix_web::{HttpResponse, Responder, get, web};
use tera::{Context, Tera};

use crate::middleware::auth::Authenticated;
use crate::routes::nav::insert_nav_context;

/// Render the home page for authenticated users.
#[get("/")]
pub async fn home(user: Option<Authenticated>, tera: web::Data<Tera>) -> impl Responder {
    // Redirect to /login if unauthenticated
    let user = match user {
        Some(u) => u,
        None => {
            return HttpResponse::Found()
                .append_header(("Location", "/login"))
                .finish();
        }
    };

    let mut context = Context::new();
    insert_nav_context(&mut context, &user);

    match tera.render("home.html", &context) {
        Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
        Err(e) => {
            eprintln!("Template error: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(home);
}
