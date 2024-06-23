use axum::response::IntoResponse;

use axum::http::StatusCode;

pub type RustusResult<T> = Result<T, RustusError>;

#[derive(thiserror::Error, Debug)]
pub enum RustusError {
    #[error("{0}")]
    Unimplemented(String),
    #[error("Not found")]
    FileNotFound,
    #[error("File already exists")]
    FileAlreadyExists,
    #[error("Given offset is incorrect.")]
    WrongOffset,
    #[error("Unknown error")]
    Unknown,
    #[error("File is frozen")]
    FrozenFile,
    #[error("Size already known")]
    SizeAlreadyKnown,
    #[error("Unable to serialize object")]
    UnableToSerialize(#[from] serde_json::Error),
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    #[error("Redis pooling error: {0}")]
    MobcError(#[from] mobc::Error<redis::RedisError>),
    #[error("Unable to get file information")]
    UnableToReadInfo,
    #[error("Unable to write file {0}")]
    UnableToWrite(String),
    #[error("Unable to remove file {0}")]
    UnableToRemove(String),
    #[error("Unable to prepare info storage. Reason: {0}")]
    UnableToPrepareInfoStorage(String),
    #[error("Unable to prepare storage. Reason: {0}")]
    UnableToPrepareStorage(String),
    #[error("Unknown extension: {0}")]
    UnknownExtension(String),
    #[error("Http request failed: {0}")]
    HttpRequestError(#[from] reqwest::Error),
    #[error("Hook invocation failed. Reason: {0}")]
    HookError(String),
    #[error("AMQP error: {0}")]
    AMQPError(#[from] lapin::Error),
    #[error("AMQP pooling error error: {0}")]
    AMQPPoolError(#[from] mobc::Error<lapin::Error>),
    #[error("Std error: {0}")]
    StdError(#[from] std::io::Error),
    #[error("Can't spawn task: {0}")]
    TokioSpawnError(#[from] tokio::task::JoinError),
    #[error("Unknown hashsum algorithm")]
    UnknownHashAlgorithm,
    #[error("Wrong checksum")]
    WrongChecksum,
    #[error("The header value is incorrect")]
    WrongHeaderValue,
    #[error("HTTP hook error. Returned status: {0}.")]
    HTTPHookError(u16, String, Option<String>),
    #[error("Found S3 error: {0}")]
    S3Error(#[from] s3::error::S3Error),
    #[error("Found invalid header: {0}")]
    InvalidHeader(#[from] axum::http::header::InvalidHeaderValue),
    #[error("HTTP error: {0}")]
    AxumHTTPError(#[from] axum::http::Error),
}

/// This conversion allows us to use `RustusError` in the `main` function.
impl From<RustusError> for std::io::Error {
    fn from(err: RustusError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, err)
    }
}

impl RustusError {
    fn get_status_code(&self) -> StatusCode {
        match self {
            RustusError::FileNotFound => StatusCode::NOT_FOUND,
            RustusError::WrongOffset => StatusCode::CONFLICT,
            RustusError::FrozenFile
            | RustusError::SizeAlreadyKnown
            | RustusError::HookError(_)
            | RustusError::UnknownHashAlgorithm
            | RustusError::WrongHeaderValue => StatusCode::BAD_REQUEST,
            RustusError::WrongChecksum => StatusCode::EXPECTATION_FAILED,
            RustusError::HTTPHookError(status, _, _) => {
                StatusCode::from_u16(*status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for RustusError {
    fn into_response(self) -> axum::response::Response {
        let status_code = self.get_status_code();
        if status_code != StatusCode::NOT_FOUND {
            tracing::error!(err=%self, "{self}");
        }
        match self {
            RustusError::HTTPHookError(_, proxy_response, content_type) => {
                axum::response::IntoResponse::into_response((
                    status_code,
                    [(
                        "Content-Type",
                        content_type.unwrap_or("text/plain; charset=utf-8".into()),
                    )],
                    proxy_response,
                ))
            }
            _ => axum::response::IntoResponse::into_response((
                status_code,
                [("Content-Type", "text/html; charset=utf-8")],
                format!("{self}"),
            )),
        }
    }
}
