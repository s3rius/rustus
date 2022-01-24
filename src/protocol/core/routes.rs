use actix_web::{web, web::Bytes, HttpRequest, HttpResponse};

use crate::errors::RustusError;
use crate::notifiers::Hook;
use crate::protocol::extensions::Extensions;
use crate::utils::headers::{check_header, parse_header};
use crate::{RustusConf, State};

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
        .finish()
}

pub async fn get_file_info(
    state: web::Data<State>,
    request: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    // Getting file id from URL.
    if request.match_info().get("file_id").is_none() {
        return Ok(HttpResponse::NotFound().body("No file id provided."));
    }
    let file_id = request.match_info().get("file_id").unwrap();

    // Getting file info from info_storage.
    let file_info = state.info_storage.get_info(file_id).await?;
    if file_info.storage != state.data_storage.to_string() {
        return Ok(HttpResponse::NotFound().body("File not found."));
    }
    let mut builder = HttpResponse::Ok();
    if file_info.is_partial {
        builder.insert_header(("Upload-Concat", "partial"));
    }
    if file_info.is_final && file_info.parts.is_some() {
        #[allow(clippy::or_fun_call)]
        let parts = file_info
            .parts
            .clone()
            .unwrap()
            .iter()
            .map(|file| {
                format!(
                    "{}/{}",
                    state
                        .config
                        .base_url()
                        .strip_suffix('/')
                        .unwrap_or(state.config.base_url().as_str()),
                    file.as_str()
                )
            })
            .collect::<Vec<String>>()
            .join(" ");
        builder.insert_header(("Upload-Concat", format!("final; {}", parts)));
    }
    builder
        .no_chunking(file_info.offset as u64)
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
    Ok(builder.finish())
}

pub async fn write_bytes(
    request: HttpRequest,
    bytes: Bytes,
    state: web::Data<State>,
) -> actix_web::Result<HttpResponse> {
    // Checking if request has required headers.
    let check_content_type = |val: &str| val == "application/offset+octet-stream";
    if !check_header(&request, "Content-Type", check_content_type) {
        return Ok(HttpResponse::UnsupportedMediaType().body("Unknown content-type."));
    }
    // Getting current offset.
    let offset: Option<usize> = parse_header(&request, "Upload-Offset");

    if offset.is_none() {
        return Ok(HttpResponse::UnsupportedMediaType().body("No offset provided."));
    }

    if request.match_info().get("file_id").is_none() {
        return Ok(HttpResponse::NotFound().body("No file id provided."));
    }

    // New upload length.
    // Parses header `Upload-Length` only if the creation-defer-length extension is enabled.
    let updated_len = if state
        .config
        .extensions_vec()
        .contains(&Extensions::CreationDeferLength)
    {
        parse_header(&request, "Upload-Length")
    } else {
        None
    };

    let file_id = request.match_info().get("file_id").unwrap();
    // Getting file info.
    let mut file_info = state.info_storage.get_info(file_id).await?;

    // According to TUS protocol you can't update final uploads.
    if file_info.is_final {
        return Ok(HttpResponse::Forbidden().finish());
    }

    // Checking if file was stored in the same storage.
    if file_info.storage != state.data_storage.to_string() {
        return Ok(HttpResponse::NotFound().finish());
    }
    // Checking if offset from request is the same as the real offset.
    if offset.unwrap() != file_info.offset {
        return Ok(HttpResponse::Conflict().finish());
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
    state
        .data_storage
        .add_bytes(&file_info, bytes.as_ref())
        .await?;
    // Updating offset.
    file_info.offset += bytes.len();
    // Saving info to info storage.
    state.info_storage.set_info(&file_info, false).await?;

    let mut hook = Hook::PostReceive;
    if file_info.length == Some(file_info.offset) {
        hook = Hook::PostFinish;
    }
    if state.config.hook_is_active(hook) {
        let message = state
            .config
            .notification_opts
            .hooks_format
            .format(&request, &file_info)?;
        let headers = request.headers().clone();
        tokio::spawn(async move {
            state
                .notification_manager
                .send_message(message, hook, &headers)
                .await
        });
    }
    Ok(HttpResponse::NoContent()
        .insert_header(("Upload-Offset", file_info.offset.to_string()))
        .finish())
}
