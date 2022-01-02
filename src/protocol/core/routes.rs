use actix_web::{http::StatusCode, web, web::Bytes, HttpRequest, HttpResponse};

use crate::notifiers::Hook;
use crate::protocol::extensions::Extensions;
use crate::utils::headers::{check_header, parse_header};
use crate::{NotificationManager, RustusConf, Storage};

#[allow(clippy::needless_pass_by_value)]
pub fn server_info(app_conf: web::Data<RustusConf>) -> HttpResponse {
    let ext_str = app_conf
        .extensions_vec()
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(",");
    HttpResponse::Ok()
        .insert_header(("Tus-Extension", ext_str.as_str()))
        .body("")
}

pub async fn get_file_info(
    storage: web::Data<Box<dyn Storage + Send + Sync>>,
    request: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let mut builder = HttpResponse::Ok();
    if let Some(file_id) = request.match_info().get("file_id") {
        let file_info = storage.get_file_info(file_id).await?;
        builder
            .insert_header(("Upload-Offset", file_info.offset.to_string()))
            .insert_header(("Content-Length", file_info.offset.to_string()));
        // Upload length is known.
        if let Some(upload_len) = file_info.length {
            builder.insert_header(("Upload-Length", upload_len.to_string()));
        } else {
            builder.insert_header(("Upload-Defer-Length", "1"));
        }
        if let Some(meta) = file_info.get_metadata_string() {
            builder.insert_header(("Upload-Metadata", meta));
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
    notification_manager: web::Data<Box<NotificationManager>>,
    app_conf: web::Data<RustusConf>,
) -> actix_web::Result<HttpResponse> {
    if !check_header(&request, "Content-Type", "application/offset+octet-stream") {
        return Ok(HttpResponse::UnsupportedMediaType().body(""));
    }
    let offset = parse_header(&request, "Upload-Offset");

    if offset.is_none() {
        return Ok(HttpResponse::UnsupportedMediaType().body(""));
    }

    // New upload length.
    // Parses header `Upload-Length` only if the creation-defer-length extension is enabled.
    let updated_len = if app_conf
        .extensions_vec()
        .contains(&Extensions::CreationDeferLength)
    {
        parse_header(&request, "Upload-Length")
    } else {
        None
    };

    if let Some(file_id) = request.match_info().get("file_id") {
        let file_info = storage
            .add_bytes(file_id, offset.unwrap(), updated_len, bytes.as_ref())
            .await?;
        let mut hook = Hook::PostReceive;
        if file_info.length == Some(file_info.offset) {
            hook = Hook::PostFinish;
        }
        if app_conf.hook_is_active(hook) {
            let message = app_conf
                .notification_opts
                .notification_format
                .format(&request, &file_info)?;
            let headers = request.headers().clone();
            tokio::spawn(async move {
                notification_manager
                    .send_message(message, hook, &headers)
                    .await
            });
        }
        Ok(HttpResponse::NoContent()
            .insert_header(("Upload-Offset", file_info.offset.to_string()))
            .body(""))
    } else {
        Ok(HttpResponse::NotFound().body(""))
    }
}
