use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use crate::{errors::RustusResult, extensions::TusExtensions, state::RustusState};

pub async fn get_server_info(
    State(ref state): State<RustusState>,
) -> RustusResult<impl axum::response::IntoResponse> {
    let mut headers = HeaderMap::new();
    let extensions = state
        .config
        .tus_extensions
        .iter()
        .map(|ext| ext.to_string())
        .collect::<Vec<String>>()
        .join(",");

    headers.insert("Tus-Extension", extensions.parse()?);

    if state
        .config
        .tus_extensions
        .contains(&TusExtensions::Checksum)
    {
        headers.insert("Tus-Checksum-Algorithm", "md5,sha1,sha256,sha512".parse()?);
    }

    Ok((StatusCode::NO_CONTENT, headers).into_response())
}
