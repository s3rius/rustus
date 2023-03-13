use std::io::{Error, ErrorKind};

use actix_web::{http::StatusCode, HttpResponse, HttpResponseBuilder, ResponseError};
use log::error;

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
    #[cfg(feature = "db_info_storage")]
    #[error("Database error: {0}")]
    DatabaseError(#[from] rbatis::error::Error),
    #[cfg(feature = "redis_info_storage")]
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    #[cfg(feature = "redis_info_storage")]
    #[error("Redis pooling error: {0}")]
    MobcError(#[from] bb8::RunError<redis::RedisError>),
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
    #[error("Unable to configure logging: {0}")]
    LogConfigError(#[from] log::SetLoggerError),
    #[cfg(feature = "amqp_notifier")]
    #[error("AMQP error: {0}")]
    AMQPError(#[from] lapin::Error),
    #[cfg(feature = "amqp_notifier")]
    #[error("AMQP pooling error error: {0}")]
    AMQPPoolError(#[from] bb8::RunError<lapin::Error>),
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
    #[error("Metrics error: {0}")]
    PrometheusError(#[from] prometheus::Error),
    #[error("Blocking error: {0}")]
    BlockingError(#[from] actix_web::error::BlockingError),
    #[error("HTTP hook error. Returned status: {0}, Response text: {1}")]
    HTTPHookError(u16, String, Option<String>),
    #[error("Found S3 error: {0}")]
    S3Error(#[from] s3::error::S3Error),
}

/// This conversion allows us to use `RustusError` in the `main` function.
#[cfg_attr(coverage, no_coverage)]
impl From<RustusError> for Error {
    fn from(err: RustusError) -> Self {
        Error::new(ErrorKind::Other, err)
    }
}

/// Trait to convert errors to http-responses.
#[cfg_attr(coverage, no_coverage)]
impl ResponseError for RustusError {
    fn error_response(&self) -> HttpResponse {
        error!("{}", self);
        match self {
            RustusError::HTTPHookError(_, proxy_response, content_type) => {
                HttpResponseBuilder::new(self.status_code())
                    .insert_header((
                        "Content-Type",
                        content_type
                            .as_deref()
                            .unwrap_or("text/plain; charset=utf-8"),
                    ))
                    .body(proxy_response.clone())
            }
            _ => HttpResponseBuilder::new(self.status_code())
                .insert_header(("Content-Type", "text/html; charset=utf-8"))
                .body(format!("{self}")),
        }
    }

    fn status_code(&self) -> StatusCode {
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
