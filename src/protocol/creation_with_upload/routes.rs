use actix_web::dev::HttpResponseBuilder;
use actix_web::http::StatusCode;
use actix_web::web::{Buf, Bytes};
use actix_web::{web, HttpRequest, HttpResponse};

use crate::Storage;

/// Creates files with initial bytes.
///
/// This function is similar to
/// `creation:create_file`,
/// except that it can write bytes
/// right after it created a data file.
pub async fn create_file(
    storage: web::Data<Box<dyn Storage + Send + Sync>>,
    request: HttpRequest,
    bytes: Bytes,
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
    let mut upload_offset = 0;
    if !bytes.is_empty() {
        // Checking if content type matches.
        let content_type = request
            .headers()
            .get("Content-Type")
            .and_then(|header_val| match header_val.to_str() {
                Ok(val) => Some(val == "application/offset+octet-stream"),
                Err(_) => None,
            });
        if Some(true) != content_type {
            return Ok(HttpResponseBuilder::new(StatusCode::UNSUPPORTED_MEDIA_TYPE).body(""));
        }
        // Checking if
        let offset = request
            .headers()
            .get("Upload-Offset")
            .and_then(|header_val| match header_val.to_str() {
                Ok(val) => Some(String::from(val)),
                Err(_) => None,
            })
            .and_then(|val| match val.parse::<usize>() {
                Ok(offset) => Some(offset),
                Err(_) => None,
            });
        if offset.is_none() {
            return Ok(HttpResponseBuilder::new(StatusCode::UNSUPPORTED_MEDIA_TYPE).body(""));
        }

        upload_offset = storage
            .add_bytes(file_id.as_str(), offset.unwrap(), bytes.bytes())
            .await?;
    }
    let upload_url = request.url_for("core:write_bytes", &[file_id])?;
    Ok(HttpResponseBuilder::new(StatusCode::CREATED)
        .set_header("Location", upload_url.as_str())
        .set_header("Upload-Offset", upload_offset.to_string())
        .body(""))
}
