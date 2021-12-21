use std::io::{Error, ErrorKind};

use actix_web::dev::HttpResponseBuilder;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};

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
    #[error("Unable to prepare info storage. Reason: {0}")]
    UnableToPrepareInfoStorage(String),
    #[error("Unable to prepare storage. Reason: {0}")]
    UnableToPrepareStorage(String),
    #[error("Unknown extension: {0}")]
    UnknownExtension(String),
}

impl From<RustusError> for Error {
    fn from(err: RustusError) -> Self {
        Error::new(ErrorKind::Other, err)
    }
}

impl ResponseError for RustusError {
    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code())
            .set_header("Content-Type", "text/html; charset=utf-8")
            .body(format!("{}", self))
    }

    fn status_code(&self) -> StatusCode {
        match self {
            RustusError::FileNotFound => StatusCode::NOT_FOUND,
            RustusError::WrongOffset => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
