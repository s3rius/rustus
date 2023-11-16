use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{extract::State, http::HeaderMap};
use bytes::Bytes;

use crate::extensions::TusExtensions;
use crate::info_storages::base::InfoStorage;
use crate::models::file_info::FileInfo;
use crate::utils::headers::HeaderMapExt;
use crate::{errors::RustusResult, state::RustusState};

pub async fn create_route(
    State(ref state): State<RustusState>,
    headers: HeaderMap,
    _body: Bytes,
) -> RustusResult<Response> {
    let upload_len: Option<usize> = headers.parse("Upload-Length");
    if !state.config.allow_empty {
        if let Some(0) = upload_len {
            return Ok((
                StatusCode::BAD_REQUEST,
                "Upload-Length must be greater than 0",
            )
                .into_response());
        }
    }
    let defer_size = headers.check("Upload-Defer-Length", |val| val == "1");
    let defer_ext = state
        .config
        .tus_extensions_set
        .contains(&TusExtensions::CreationDeferLength);

    let is_final = headers.check("Upload-Concat", |val| val.starts_with("final;"));
    let concat_ext = state
        .config
        .tus_extensions_set
        .contains(&TusExtensions::Concatenation);

    if upload_len.is_none() && !((defer_ext && defer_size) || (concat_ext && is_final)) {
        return Ok((StatusCode::BAD_REQUEST, "Upload-Length is required").into_response());
    }

    if state.config.max_file_size.is_some() && state.config.max_file_size < upload_len {
        return Ok((
            StatusCode::BAD_REQUEST,
            format!(
                "Upload-Length should be less than or equal to {}",
                state.config.max_file_size.unwrap()
            ),
        )
            .into_response());
    }

    let meta = headers.get_metadata();

    let file_id = uuid::Uuid::new_v4().to_string();
    let mut file_info = FileInfo::new(file_id.as_str(), upload_len, None, "storage".into(), meta);

    let is_partial = headers.check("Upload-Concat", |val| val == "partial");

    if concat_ext {
        if is_final {
            file_info.is_final = true;
            let upload_parts = headers.get_upload_parts();
            if upload_parts.is_empty() {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Upload-Concat header has no parts to create final upload.",
                )
                    .into_response());
            }
            file_info.parts = Some(upload_parts);
            file_info.deferred_size = false;
        }
        if is_partial {
            file_info.is_partial = true;
        }
    }

    state.info_storage.set_info(&file_info, true).await?;

    Ok(().into_response())
}
