use axum::{
    extract::Path,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use crate::{
    errors::{RustusError, RustusResult},
    utils::headers::HeaderMapExt,
};

pub async fn upload_chunk_route(
    Path(upload_id): Path<String>,
    headers: HeaderMap,
) -> RustusResult<axum::response::Response> {
    println!("hehehe {}", upload_id);
    if !headers.check("Content-Type", |val| {
        val == "application/offset+octet-stream"
    }) {
        return Ok((StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported media type").into_response());
    }

    let offset: Option<usize> = headers.parse("Upload-Offset");
    if offset.is_none() {
        return Err(RustusError::MissingOffset);
    }

    Ok("upload_chunk_route".into_response())
}
