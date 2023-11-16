use axum::response::IntoResponse;

pub type RustusResult<T> = Result<T, RustusError>;

#[derive(Debug, thiserror::Error)]
pub enum RustusError {
    #[error("Unable to prepare info storage: {0}")]
    UnableToRemove(String),
    #[error("File not found.")]
    FileNotFound,
    #[error("Something bad happened: {0}")]
    AnyHowShit(#[from] anyhow::Error),
}

impl IntoResponse for RustusError {
    fn into_response(self) -> axum::response::Response {
        axum::response::IntoResponse::into_response(self.to_string())
    }
}
