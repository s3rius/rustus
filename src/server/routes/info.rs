use axum::{extract::State, http::StatusCode};

use crate::state::RustusState;

pub async fn info_route(State(ref state): State<RustusState>) -> impl axum::response::IntoResponse {
    let extensions = state
        .config
        .tus_extensions
        .iter()
        .map(|ext| ext.to_string())
        .collect::<Vec<String>>()
        .join(",");

    (StatusCode::NO_CONTENT, [("Tus-Extension", extensions)])
}
