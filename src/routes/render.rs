use actix_web::HttpResponse;
use tera::{Context, Tera};

/// Render a Tera template into an HTML HttpResponse.
pub fn render(tera: &Tera, name: &str, context: &Context) -> HttpResponse {
    match tera.render(name, context) {
        Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
        Err(e) => {
            eprintln!("Template error: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
