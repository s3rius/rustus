use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::IntoResponse,
};
use bytes::Bytes;
use tracing::Instrument;

use crate::{
    data_storage::base::Storage,
    errors::{RustusError, RustusResult},
    extensions::TusExtensions,
    info_storages::base::InfoStorage,
    notifiers::hooks::Hook,
    state::RustusState,
    utils::{hashes::verify_chunk_checksum, headers::HeaderMapExt},
};

#[tracing::instrument(level = "info", skip_all, fields(upload_id = tracing::field::Empty))]
pub async fn handler(
    uri: Uri,
    method: Method,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<RustusState>>,
    Path(upload_id): Path<String>,
    body: Bytes,
) -> RustusResult<axum::response::Response> {
    tracing::Span::current().record("upload_id", &upload_id.as_str());
    if !headers.check("Content-Type", |val| {
        val == "application/offset+octet-stream"
    }) {
        return Ok((StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported media type").into_response());
    }

    let offset: Option<usize> = headers.parse("Upload-Offset");
    if offset.is_none() {
        return Ok((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Missing Upload-Offset header",
        )
            .into_response());
    }

    if state
        .config
        .tus_extensions_set
        .contains(&TusExtensions::Checksum)
    {
        if let Some(check_sum) = headers.get("Upload-Checksum") {
            if !verify_chunk_checksum(check_sum, &body)? {
                return Err(RustusError::WrongChecksum);
            }
        }
    }

    // Getting file info.
    let mut file_info = state.info_storage.get_info(&upload_id).await?;
    // According to TUS protocol you can't update final uploads.
    if file_info.is_final {
        return Ok((StatusCode::FORBIDDEN, "The upload is finished").into_response());
    }

    // Checking if file was stored in the same storage.
    if file_info.storage != state.data_storage.get_name() {
        return Err(RustusError::FileNotFound);
    }

    // Checking if offset from request is the same as the real offset.
    if offset.unwrap() != file_info.offset {
        return Err(RustusError::WrongOffset);
    }

    // New upload length.
    // Parses header `Upload-Length` only if the creation-defer-length extension is enabled.
    let updated_len = if state
        .config
        .tus_extensions
        .contains(&TusExtensions::CreationDeferLength)
    {
        headers.parse("Upload-Length")
    } else {
        None
    };

    if let Some(new_len) = updated_len {
        // Whoop, someone gave us total file length
        // less that he had already uploaded.
        if new_len < file_info.offset {
            return Err(RustusError::WrongOffset);
        }
        // We already know the exact size of a file.
        // Someone want to update it.
        // Anyway, it's not allowed, heh.
        if file_info.length.is_some() {
            return Err(RustusError::SizeAlreadyKnown);
        }

        // All checks are ok. Now our file will have exact size.
        file_info.deferred_size = false;
        file_info.length = Some(new_len);
    }

    // Checking if the size of the upload is already equals
    // to calculated offset. It means that all bytes were already written.
    if Some(file_info.offset) == file_info.length {
        return Err(RustusError::FrozenFile);
    }
    let chunk_len = body.len();

    // Appending bytes to file.
    state.data_storage.add_bytes(&file_info, body).await?;
    // bytes.clear()
    // Updating offset.
    file_info.offset += chunk_len;
    // Saving info to info storage.
    state.info_storage.set_info(&file_info, false).await?;

    let mut hook = Hook::PostReceive;

    if file_info.length == Some(file_info.offset) {
        hook = Hook::PostFinish;
    }

    if state.config.notification_hooks_set.contains(&hook) {
        let state_clone = state.clone();
        let msg = state.config.notification_config.hooks_format.format(
            &uri,
            &method,
            &addr,
            &headers,
            state.config.behind_proxy,
            &file_info,
        );
        let headers_clone = headers.clone();

        tokio::spawn(
            async move {
                state_clone
                    .notificator
                    .notify_all(msg, hook, &headers_clone)
                    .await
                    .ok();
            }
            .in_current_span(),
        );
    }

    Ok((
        StatusCode::NO_CONTENT,
        [("Upload-Offset", file_info.offset.to_string())],
    )
        .into_response())
}
