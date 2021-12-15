use actix_web::{HttpRequest, HttpResponse};
use actix_web::dev::HttpResponseBuilder;
use actix_web::http::StatusCode;
use actix_web::web;
use actix_web::web::{Buf, Bytes};

use crate::storages::Storage;

/// Default response to all unknown URLs.
#[allow(clippy::needless_pass_by_value)]
pub fn not_found() -> HttpResponse {
    HttpResponseBuilder::new(StatusCode::NOT_FOUND)
        .set_header("Content-Type", "text/html; charset=utf-8")
        .body("Not found")
}

pub fn server_info() -> HttpResponse {
    HttpResponseBuilder::new(StatusCode::OK)
        .set_header("Tus-Extension", "creation")
        .body("")
}

pub async fn get_file_info<T: Storage>(
    storage: web::Data<T>,
    request: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let resp = if let Some(file_id) = request.match_info().get("file_id") {
        let file_info = storage.get_file_info(file_id).await?;
        HttpResponseBuilder::new(StatusCode::OK)
            .set_header("Upload-Offset", file_info.offset.to_string())
            .set_header("Upload-Length", file_info.length.to_string())
            .body("")
    } else {
        HttpResponseBuilder::new(StatusCode::NOT_FOUND).body("")
    };
    Ok(resp)
}

pub async fn create_file<T: Storage>(
    storage: web::Data<T>,
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
    let upload_url = request.url_for("write_bytes", &[file_id])?;
    Ok(HttpResponseBuilder::new(StatusCode::CREATED)
        .set_header("Location", upload_url.as_str())
        .body(""))
}

pub async fn write_bytes<T: Storage>(
    request: HttpRequest,
    bytes: Bytes,
    storage: web::Data<T>,
) -> actix_web::Result<HttpResponse> {
    let conflict_response = HttpResponseBuilder::new(StatusCode::UNSUPPORTED_MEDIA_TYPE).body("");
    let content_type =
        request
            .headers()
            .get("Content-Type")
            .and_then(|header_val| match header_val.to_str() {
                Ok(val) => Some(val == "application/offset+octet-stream"),
                Err(_) => None,
            });
    if Some(true) != content_type {
        return Ok(conflict_response);
    }
    let offset =
        request
            .headers()
            .get("Upload-Offset")
            .and_then(|header_val| match header_val.to_str() {
                Ok(val) => Some(String::from(val)),
                Err(_) => None,
            }).and_then(|val| {
            match val.parse::<usize>() {
                Ok(offset) => Some(offset),
                Err(_) => None
            }
        });
    if offset.is_none() {
        return Ok(conflict_response);
    }
    if let Some(file_id) = request.match_info().get("file_id") {
        let offset = storage.add_bytes(file_id, offset.unwrap(), bytes.bytes()).await?;
        Ok(HttpResponseBuilder::new(StatusCode::NO_CONTENT)
            .set_header("Upload-Offset", offset.to_string())
            .body(""))
    } else {
        Ok(HttpResponseBuilder::new(StatusCode::NOT_FOUND).body(""))
    }
}
