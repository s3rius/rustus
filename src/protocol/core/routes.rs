use actix_web::{
    dev::HttpResponseBuilder,
    http::StatusCode,
    web,
    web::{Buf, Bytes},
    HttpRequest, HttpResponse,
};

use crate::{Storage, TuserConf};

#[allow(clippy::needless_pass_by_value)]
pub fn server_info(app_conf: web::Data<TuserConf>) -> HttpResponse {
    let ext_str = app_conf
        .extensions_vec()
        .into_iter()
        .map(String::from)
        .collect::<Vec<String>>()
        .join(",");
    HttpResponseBuilder::new(StatusCode::OK)
        .set_header("Tus-Extension", ext_str.as_str())
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

pub async fn write_bytes<T: Storage>(
    request: HttpRequest,
    bytes: Bytes,
    storage: web::Data<T>,
) -> actix_web::Result<HttpResponse> {
    let content_type =
        request
            .headers()
            .get("Content-Type")
            .and_then(|header_val| match header_val.to_str() {
                Ok(val) => Some(val == "application/offset+octet-stream"),
                Err(_) => None,
            });
    if Some(true) != content_type {
        return Ok(HttpResponseBuilder::new(StatusCode::UNSUPPORTED_MEDIA_TYPE).body(""));
    }
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
    if let Some(file_id) = request.match_info().get("file_id") {
        let offset = storage
            .add_bytes(file_id, offset.unwrap(), bytes.bytes())
            .await?;
        Ok(HttpResponseBuilder::new(StatusCode::NO_CONTENT)
            .set_header("Upload-Offset", offset.to_string())
            .body(""))
    } else {
        Ok(HttpResponseBuilder::new(StatusCode::NOT_FOUND).body(""))
    }
}
