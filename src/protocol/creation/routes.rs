use actix_web::dev::HttpResponseBuilder;
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse};

use crate::Storage;

pub async fn create_file(
    storage: web::Data<Box<dyn Storage + Send + Sync>>,
    request: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let length = request
        .headers()
        .get("Upload-Length")
        .and_then(|value| match value.to_str() {
            Ok(header_str) => Some(String::from(header_str)),
            Err(_) => None,
        })
        .and_then(|val| match val.parse::<usize>() {
            Ok(num) => Some(num),
            Err(_) => None,
        });
    let file_id = storage.create_file(length, None).await?;
    let upload_url = request.url_for("core:write_bytes", &[file_id])?;
    Ok(HttpResponseBuilder::new(StatusCode::CREATED)
        .set_header("Location", upload_url.as_str())
        .set_header("Upload-Offset", "0")
        .body(""))
}
