use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;

use crate::{
    data_storage::base::Storage,
    errors::{RustusError, RustusResult},
    extensions::TusExtensions,
    info_storages::base::InfoStorage,
    state::RustusState,
};

pub async fn handler(
    State(state): State<Arc<RustusState>>,
    Path(upload_id): Path<String>,
) -> RustusResult<Response> {
    if !state
        .config
        .tus_extensions_set
        .contains(&TusExtensions::Getting)
    {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    let file_info = state.info_storage.get_info(&upload_id).await?;
    if file_info.storage != state.data_storage.get_name() {
        return Err(RustusError::FileNotFound);
    }

    state.data_storage.get_contents(&file_info).await
}
