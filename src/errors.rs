use std::io::{Error, ErrorKind};

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, HttpResponseBuilder, ResponseError};
use log::error;

pub type RustusResult<T> = Result<T, RustusError>;

#[derive(thiserror::Error, Debug)]
pub enum RustusError {
    #[error("Not found")]
    FileNotFound,
    #[error("File with id {0} already exists")]
    FileAlreadyExists(String),
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
    RedisError(#[from] mobc_redis::redis::RedisError),
    #[cfg(feature = "redis_info_storage")]
    #[error("Redis error: {0}")]
    MobcError(#[from] mobc_redis::mobc::Error<mobc_redis::redis::RedisError>),
    #[error("Unable to get file information")]
    UnableToReadInfo,
    #[error("Unable to write file {0}")]
    UnableToWrite(String),
    #[error("Unable to remove file {0}")]
    UnableToRemove(String),
    #[error("Unable to resize file {0}")]
    UnableToResize(String),
    #[error("Unable to seek in file {0}")]
    UnableToSeek(String),
    #[error("Unable to prepare info storage. Reason: {0}")]
    UnableToPrepareInfoStorage(String),
    #[error("Unable to prepare storage. Reason: {0}")]
    UnableToPrepareStorage(String),
    #[error("Unknown extension: {0}")]
    UnknownExtension(String),
    #[cfg(feature = "http_notifier")]
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
    #[error("AMQP error: {0}")]
    AMQPPoolError(#[from] mobc_lapin::mobc::Error<lapin::Error>),
    #[error("Std error: {0}")]
    StdError(#[from] std::io::Error),
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
        HttpResponseBuilder::new(self.status_code())
            .insert_header(("Content-Type", "text/html; charset=utf-8"))
            .body(format!("{}", self))
    }

    fn status_code(&self) -> StatusCode {
        match self {
            RustusError::FileNotFound => StatusCode::NOT_FOUND,
            RustusError::WrongOffset => StatusCode::CONFLICT,
            RustusError::FrozenFile | RustusError::SizeAlreadyKnown | RustusError::HookError(_) => {
                StatusCode::BAD_REQUEST
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
