use actix_web::{web, web::Bytes, HttpRequest, HttpResponse};

use crate::errors::RustusError;
use crate::notifiers::Hook;
use crate::protocol::extensions::Extensions;
use crate::utils::headers::{check_header, parse_header};
use crate::{InfoStorage, NotificationManager, RustusConf, Storage};

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
    info_storage: web::Data<Box<dyn InfoStorage + Send + Sync>>,
    storage: web::Data<Box<dyn Storage + Send + Sync>>,
    request: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    // Getting file id from URL.
    if request.match_info().get("file_id").is_none() {
        return Ok(HttpResponse::NotFound().body(""));
    }
    let file_id = request.match_info().get("file_id").unwrap();

    // Getting file info from info_storage.
    let file_info = info_storage.get_info(file_id).await?;
    if file_info.storage != storage.to_string() {
        return Ok(HttpResponse::NotFound().body(""));
    }
    let mut builder = HttpResponse::Ok();
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
    Ok(builder.body(""))
}

pub async fn write_bytes(
    request: HttpRequest,
    bytes: Bytes,
    storage: web::Data<Box<dyn Storage + Send + Sync>>,
    info_storage: web::Data<Box<dyn InfoStorage + Send + Sync>>,
    notification_manager: web::Data<Box<NotificationManager>>,
    app_conf: web::Data<RustusConf>,
) -> actix_web::Result<HttpResponse> {
    // Checking if request has required headers.
    if !check_header(&request, "Content-Type", "application/offset+octet-stream") {
        return Ok(HttpResponse::UnsupportedMediaType().body(""));
    }
    // Getting current offset.
    let offset: Option<usize> = parse_header(&request, "Upload-Offset");

    if offset.is_none() {
        return Ok(HttpResponse::UnsupportedMediaType().body(""));
    }

    if request.match_info().get("file_id").is_none() {
        return Ok(HttpResponse::NotFound().body(""));
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

    let file_id = request.match_info().get("file_id").unwrap();
    // Getting file info.
    let mut file_info = info_storage.get_info(file_id).await?;

    // Checking if file was stored in the same storage.
    if file_info.storage != storage.to_string() {
        return Ok(HttpResponse::NotFound().body(""));
    }
    // Checking if offset from request is the same as the real offset.
    if offset.unwrap() != file_info.offset {
        return Ok(HttpResponse::Conflict().body(""));
    }

    // If someone want to update file length.
    // This required by Upload-Defer-Length extension.
    if let Some(new_len) = updated_len {
        // Whoop, someone gave us total file length
        // less that he had already uploaded.
        if new_len < file_info.offset {
            return Err(RustusError::WrongOffset.into());
        }
        // We already know the exact size of a file.
        // Someone want to update it.
        // Anyway, it's not allowed, heh.
        if file_info.length.is_some() {
            return Err(RustusError::SizeAlreadyKnown.into());
        }

        // All checks are ok. Now our file will have exact size.
        file_info.deferred_size = false;
        file_info.length = Some(new_len);
    }

    // Checking if the size of the upload is already equals
    // to calculated offset. It means that all bytes were already written.
    if Some(file_info.offset) == file_info.length {
        return Err(RustusError::FrozenFile.into());
    }

    // Appending bytes to file.
    storage.add_bytes(&file_info, bytes.as_ref()).await?;
    // Updating offset.
    file_info.offset += bytes.len();
    // Saving info to info storage.
    info_storage.set_info(&file_info, false).await?;

    let mut hook = Hook::PostReceive;
    if file_info.length == Some(file_info.offset) {
        hook = Hook::PostFinish;
    }
    if app_conf.hook_is_active(hook) {
        let message = app_conf
            .notification_opts
            .hooks_format
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
}
