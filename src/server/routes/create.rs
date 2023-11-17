use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;

use crate::{
    data_storage::base::Storage, errors::RustusResult, extensions::TusExtensions,
    info_storages::base::InfoStorage, models::file_info::FileInfo, state::RustusState,
    utils::headers::HeaderMapExt,
};

pub async fn create_upload(
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
    let mut file_info = FileInfo::new(
        file_id.as_str(),
        upload_len,
        None,
        state.data_storage.get_name().to_string(),
        meta,
    );

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

    file_info.path = Some(state.data_storage.create_file(&file_info).await?);

    if file_info.is_final {
        let mut final_size = 0;
        let mut parts_info = Vec::new();
        for part_id in file_info.clone().parts.unwrap() {
            let part = state.info_storage.get_info(part_id.as_str()).await?;
            if part.length != Some(part.offset) {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    format!("{} upload is not complete.", part.id),
                )
                    .into_response());
            }
            if !part.is_partial {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    format!("{} upload is not partial.", part.id),
                )
                    .into_response());
            }
            final_size += &part.length.unwrap();
            parts_info.push(part.clone());
        }
        state
            .data_storage
            .concat_files(&file_info, parts_info.clone())
            .await?;
        file_info.offset = final_size;
        file_info.length = Some(final_size);
        if state.config.remove_parts {
            for part in parts_info {
                state.data_storage.remove_file(&part).await?;
                state.info_storage.remove_info(part.id.as_str()).await?;
            }
        }
    }

    state.info_storage.set_info(&file_info, true).await?;
    let upload_url = state.config.get_url(&file_info.id);

    Ok((
        StatusCode::CREATED,
        [
            ("Location", upload_url.as_str()),
            ("Upload-Offset", file_info.offset.to_string().as_str()),
        ],
    )
        .into_response())
}
