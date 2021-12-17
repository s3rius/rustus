use actix_web::dev::HttpResponseBuilder;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;

/// Default response to all unknown URLs.
pub fn not_found() -> HttpResponse {
    HttpResponseBuilder::new(StatusCode::NOT_FOUND)
        .set_header("Content-Type", "text/html; charset=utf-8")
        .body("Not found")
}
