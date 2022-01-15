use std::collections::HashMap;

use actix_web::web::Bytes;
use actix_web::{web, HttpRequest, HttpResponse};

use crate::info_storages::FileInfo;
use crate::notifiers::Hook;
use crate::protocol::extensions::Extensions;
use crate::utils::headers::{check_header, parse_header};
use crate::State;

/// Get metadata info from request.
///
/// Metadata is located in Upload-Metadata header.
/// Key and values are separated by spaces and
/// pairs are delimited with commas.
///
/// E.G.
/// `Upload-Metadata: Video bWVtZXM=,Category bWVtZXM=`
///
/// All values are encoded as base64 strings.
fn get_metadata(request: &HttpRequest) -> Option<HashMap<String, String>> {
    request
        .headers()
        .get("Upload-Metadata")
        .and_then(|her| match her.to_str() {
            Ok(str_val) => Some(String::from(str_val)),
            Err(_) => None,
        })
        .map(|header_string| {
            let mut meta_map = HashMap::new();
            for meta_pair in header_string.split(',') {
                let mut split = meta_pair.split(' ');
                let key = split.next();
                let b64val = split.next();
                if key.is_none() || b64val.is_none() {
                    continue;
                }
                let value =
                    base64::decode(b64val.unwrap()).map(|value| match String::from_utf8(value) {
                        Ok(val) => Some(val),
                        Err(_) => None,
                    });
                if let Ok(Some(res)) = value {
                    meta_map.insert(String::from(key.unwrap()), res);
                }
            }
            meta_map
        })
}

/// Create file.
///
/// This method allows you to create file to start uploading.
///
/// This method supports defer-length if
/// you don't know actual file length and
/// you can upload first bytes if creation-with-upload
/// extension is enabled.
pub async fn create_file(
    state: web::Data<State>,
    request: HttpRequest,
    bytes: Bytes,
) -> actix_web::Result<HttpResponse> {
    // Getting Upload-Length header value as usize.
    let length = parse_header(&request, "Upload-Length");
    // Checking Upload-Defer-Length header.
    let defer_size = check_header(&request, "Upload-Defer-Length", "1");

    // Indicator that creation-defer-length is enabled.
    let defer_ext = state
        .config
        .extensions_vec()
        .contains(&Extensions::CreationDeferLength);

    // Check that Upload-Length header is provided.
    // Otherwise checking that defer-size feature is enabled
    // and header provided.
    if length.is_none() && (defer_ext && !defer_size) {
        return Ok(HttpResponse::BadRequest().body(""));
    }

    let meta = get_metadata(&request);

    let file_id = uuid::Uuid::new_v4().to_string();
    let mut file_info = FileInfo::new(
        file_id.as_str(),
        length,
        None,
        state.data_storage.to_string(),
        meta.clone(),
    );

    if state.config.hook_is_active(Hook::PreCreate) {
        let message = state
            .config
            .notification_opts
            .hooks_format
            .format(&request, &file_info)?;
        let headers = request.headers();
        state
            .notification_manager
            .send_message(message, Hook::PreCreate, headers)
            .await?;
    }

    // Create file and get the it's path.
    file_info.path = Some(state.data_storage.create_file(&file_info).await?);

    // Create upload URL for this file.
    let upload_url = request.url_for("core:write_bytes", &[file_info.id.clone()])?;

    // Checking if creation-with-upload extension is enabled.
    let with_upload = state
        .config
        .extensions_vec()
        .contains(&Extensions::CreationWithUpload);
    if with_upload && !bytes.is_empty() {
        if !check_header(&request, "Content-Type", "application/offset+octet-stream") {
            return Ok(HttpResponse::BadRequest().body(""));
        }
        // Writing first bytes.
        state
            .data_storage
            .add_bytes(&file_info, bytes.as_ref())
            .await?;
        file_info.offset += bytes.len();
    }

    state.info_storage.set_info(&file_info, true).await?;

    if state.config.hook_is_active(Hook::PostCreate) {
        let message = state
            .config
            .notification_opts
            .hooks_format
            .format(&request, &file_info)?;
        let headers = request.headers().clone();
        // Adding send_message task to tokio reactor.
        // Thin function would be executed in background.
        tokio::spawn(async move {
            state
                .notification_manager
                .send_message(message, Hook::PostCreate, &headers)
                .await
        });
    }

    Ok(HttpResponse::Created()
        .insert_header(("Location", upload_url.as_str()))
        .insert_header(("Upload-Offset", file_info.offset.to_string()))
        .body(""))
}
