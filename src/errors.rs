use axum::response::IntoResponse;

pub type RustusResult<T> = Result<T, RustusError>;

#[derive(Debug, thiserror::Error)]
pub enum RustusError {
    #[error("Unable to prepare info storage: {0}")]
    UnableToRemove(String),
    #[error("Cannot write: {0}")]
    UnableToWrite(String),
    #[error("File not found.")]
    FileNotFound,
    #[error("Something really bad happened: {0}")]
    AnyHowError(#[from] anyhow::Error),
    #[error("Unimplemented: {0}")]
    Unimplemented(String),
}

impl IntoResponse for RustusError {
    fn into_response(self) -> axum::response::Response {
        axum::response::IntoResponse::into_response(self.to_string())
    }
}
