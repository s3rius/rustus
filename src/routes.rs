use actix_web::HttpResponse;

/// Default response to all unknown URLs.
/// All protocol urls can be found
/// at `crate::protocol::*`.
#[allow(clippy::unused_async)]

pub async fn not_found() -> HttpResponse {
    HttpResponse::NotFound().finish()
}

/// Checks that application is accepting connections correctly.
#[allow(clippy::unused_async)]

pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
