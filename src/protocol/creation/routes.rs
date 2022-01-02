use std::collections::HashMap;

use actix_web::web::Bytes;
use actix_web::{web, HttpRequest, HttpResponse};

use crate::info_storages::FileInfo;
use crate::notifiers::Hook;
use crate::protocol::extensions::Extensions;
use crate::utils::headers::{check_header, parse_header};
use crate::{NotificationManager, RustusConf, Storage};

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

pub async fn create_file(
    storage: web::Data<Box<dyn Storage + Send + Sync>>,
    notification_manager: web::Data<Box<NotificationManager>>,
    app_conf: web::Data<RustusConf>,
    request: HttpRequest,
    bytes: Bytes,
) -> actix_web::Result<HttpResponse> {
    // Getting Upload-Length header value as usize.
    let length = parse_header(&request, "Upload-Length");
    // Checking Upload-Defer-Length header.
    let defer_size = check_header(&request, "Upload-Defer-Length", "1");

    // Indicator that creation-defer-length is enabled.
    let defer_ext = app_conf
        .extensions_vec()
        .contains(&Extensions::CreationDeferLength);

    // Check that Upload-Length header is provided.
    // Otherwise checking that defer-size feature is enabled
    // and header provided.
    if length.is_none() && (defer_ext && !defer_size) {
        return Ok(HttpResponse::BadRequest().body(""));
    }

    let meta = get_metadata(&request);

    if app_conf.hook_is_active(Hook::PreCreate) {
        let initial_file_info = FileInfo::new("", length, None, meta.clone());
        let message = app_conf
            .notification_opts
            .notification_format
            .format(&request, &initial_file_info)?;
        let headers = request.headers();
        notification_manager
            .send_message(message, Hook::PreCreate, headers)
            .await?;
    }

    // Create file and get the id.
    let mut file_info = storage.create_file(length, meta).await?;

    // Create upload URL for this file.
    let upload_url = request.url_for("core:write_bytes", &[file_info.id.clone()])?;

    // Checking if creation-with-upload extension is enabled.
    let with_upload = app_conf
        .extensions_vec()
        .contains(&Extensions::CreationWithUpload);
    if with_upload && !bytes.is_empty() {
        if !check_header(&request, "Content-Type", "application/offset+octet-stream") {
            return Ok(HttpResponse::BadRequest().body(""));
        }
        // Writing first bytes.
        file_info = storage
            .add_bytes(file_info.id.as_str(), 0, None, bytes.as_ref())
            .await?;
    }

    if app_conf.hook_is_active(Hook::PostCreate) {
        let message = app_conf
            .notification_opts
            .notification_format
            .format(&request, &file_info)?;
        let headers = request.headers().clone();
        // Adding send_message task to tokio reactor.
        // Thin function would be executed in background.
        tokio::spawn(async move {
            notification_manager
                .send_message(message, Hook::PostCreate, &headers)
                .await
        });
    }

    Ok(HttpResponse::Created()
        .insert_header(("Location", upload_url.as_str()))
        .insert_header(("Upload-Offset", file_info.offset.to_string()))
        .body(""))
}
