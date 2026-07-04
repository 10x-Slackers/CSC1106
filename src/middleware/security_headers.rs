//! Security headers middleware for Actix-web.
//!
//! Authors: commit2main

use actix_web::Error;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;

/// Adds security headers to the response.
pub async fn security_headers<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<B>, Error>
where
    B: MessageBody + 'static,
{
    let mut res = next.call(req).await?;
    let headers = res.headers_mut();
    headers.insert(
        actix_web::http::header::X_CONTENT_TYPE_OPTIONS,
        actix_web::http::header::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        actix_web::http::header::X_FRAME_OPTIONS,
        actix_web::http::header::HeaderValue::from_static("DENY"),
    );
    headers.insert(
        actix_web::http::header::REFERRER_POLICY,
        actix_web::http::header::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        actix_web::http::header::CONTENT_SECURITY_POLICY,
        actix_web::http::header::HeaderValue::from_static("default-src 'self'"),
    );
    Ok(res)
}
