use actix_web::{
    dev::HttpResponseBuilder,
    http::StatusCode,
    web,
    web::{Buf, Bytes},
    HttpRequest, HttpResponse,
};

use crate::utils::headers::{check_header, parse_header};
use crate::{RustusConf, Storage};

#[allow(clippy::needless_pass_by_value)]
pub fn server_info(app_conf: web::Data<RustusConf>) -> HttpResponse {
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

pub async fn get_file_info(
    storage: web::Data<Box<dyn Storage + Send + Sync>>,
    request: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let mut builder = HttpResponseBuilder::new(StatusCode::OK);
    if let Some(file_id) = request.match_info().get("file_id") {
        let file_info = storage.get_file_info(file_id).await?;
        builder
            .set_header("Upload-Offset", file_info.offset.to_string())
            .set_header("Upload-Length", file_info.length.to_string())
            .set_header("Content-Length", file_info.offset.to_string());
        if file_info.deferred_size {
            builder.set_header("Upload-Defer-Length", "1");
        }
    } else {
        builder.status(StatusCode::NOT_FOUND);
    };
    Ok(builder.body(""))
}

pub async fn write_bytes(
    request: HttpRequest,
    bytes: Bytes,
    storage: web::Data<Box<dyn Storage + Send + Sync>>,
) -> actix_web::Result<HttpResponse> {
    if !check_header(&request, "Content-Type", "application/offset+octet-stream") {
        return Ok(HttpResponseBuilder::new(StatusCode::UNSUPPORTED_MEDIA_TYPE).body(""));
    }
    let offset = parse_header(&request, "Upload-Offset");

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
