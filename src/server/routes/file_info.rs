use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::Response,
};
use reqwest::StatusCode;

use crate::{
    data_storage::base::Storage, errors::RustusResult, info_storages::base::InfoStorage,
    state::RustusState,
};

pub async fn handler(
    State(state): State<Arc<RustusState>>,
    Path(upload_id): Path<String>,
) -> RustusResult<Response> {
    let file_info = state.info_storage.get_info(&upload_id).await?;
    if file_info.storage != state.data_storage.get_name() {
        return Err(crate::errors::RustusError::FileNotFound);
    }
    let mut response = Response::builder().status(StatusCode::OK);

    if file_info.is_partial {
        response = response.header("Upload-Concat", "partial");
    }
    if file_info.is_final && file_info.parts.is_some() {
        let parts = file_info
            .parts
            .as_ref()
            .unwrap()
            .iter()
            .map(|file| format!("/{}/{}", state.config.url, file.as_str()))
            .collect::<Vec<String>>()
            .join(" ");
        response = response.header("Upload-Concat", format!("final; {parts}"));
    }
    response = response.header("Upload-Offset", file_info.offset.to_string());
    if let Some(upload_len) = file_info.length {
        response = response
            .header("Content-Length", file_info.offset.to_string())
            .header("Upload-Length", upload_len.to_string());
    } else {
        response = response.header("Upload-Defer-Length", "1");
    }
    if let Some(meta) = file_info.get_metadata_string() {
        response = response.header("Upload-Metadata", meta);
    }
    response = response.header(
        "Upload-Created",
        &file_info.created_at.timestamp().to_string(),
    );
    Ok(response.body(axum::body::Body::empty())?)
}
