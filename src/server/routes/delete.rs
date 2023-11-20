use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use http::{HeaderMap, Method, Uri};

use crate::{
    data_storage::base::Storage,
    errors::{RustusError, RustusResult},
    extensions::TusExtensions,
    info_storages::base::InfoStorage,
    notifiers::hooks::Hook,
    state::RustusState,
    utils::result::MonadLogger,
};

pub async fn handler(
    uri: Uri,
    method: Method,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<RustusState>>,
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

    if state
        .config
        .notification_hooks_set
        .contains(&Hook::PreTerminate)
    {
        state
            .notificator
            .send_message(
                state.config.notification_config.hooks_format.format(
                    &uri,
                    &method,
                    &addr,
                    &headers,
                    state.config.behind_proxy,
                    &file_info,
                ),
                Hook::PreTerminate,
                &headers,
            )
            .await?;
    }

    state.data_storage.remove_file(&file_info).await?;
    state.info_storage.remove_info(&file_info.id).await?;

    if state
        .config
        .notification_hooks_set
        .contains(&Hook::PostTerminate)
    {
        let msg = state.config.notification_config.hooks_format.format(
            &uri,
            &method,
            &addr,
            &headers,
            state.config.behind_proxy,
            &file_info,
        );
        let state_cln = state.clone();
        tokio::spawn(async move {
            state_cln
                .notificator
                .send_message(msg, Hook::PostTerminate, &headers)
                .await
                .mlog_warn("Cannot send PostTerminate hook")
        });
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}
