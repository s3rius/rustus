use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    data_storage::base::Storage,
    errors::{RustusError, RustusResult},
    extensions::TusExtensions,
    info_storages::base::InfoStorage,
    state::RustusState,
};

pub async fn delete_upload(
    State(state): State<RustusState>,
    Path(upload_id): Path<String>,
) -> RustusResult<Response> {
    if !state
        .config
        .tus_extensions_set
        .contains(&TusExtensions::Termination)
    {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    let file_info = state.info_storage.get_info(&upload_id).await?;
    if file_info.storage != state.data_storage.get_name() {
        return Err(RustusError::FileNotFound);
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}
