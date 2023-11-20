use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use crate::{errors::RustusResult, extensions::TusExtensions, state::RustusState};

pub async fn handler(
    State(ref state): State<Arc<RustusState>>,
) -> RustusResult<impl axum::response::IntoResponse> {
    let mut headers = HeaderMap::new();
    let extensions = state
        .config
        .tus_extensions
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join(",");

    headers.insert("tus-extension", extensions.parse()?);

    if state
        .config
        .tus_extensions
        .contains(&TusExtensions::Checksum)
    {
        headers.insert("tus-checksum-algorithm", "md5,sha1,sha256,sha512".parse()?);
    }

    Ok((StatusCode::NO_CONTENT, headers).into_response())
}
