use std::io::{Error, ErrorKind};

use actix_web::dev::HttpResponseBuilder;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};

pub type TuserResult<T> = Result<T, TuserError>;

#[derive(thiserror::Error, Debug)]
pub enum TuserError {
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
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Unable to get file information")]
    UnableToReadInfo,
    #[error("Unable to write file {0}")]
    UnableToWrite(String),
    #[error("Unable to remove file {0}")]
    UnableToRemove(String),
    #[error("Unable to prepare storage. Reason: {0}")]
    UnableToPrepareStorage(String),
    #[error("Unknown extension: {0}")]
    UnknownExtension(String),
}

impl From<TuserError> for Error {
    fn from(err: TuserError) -> Self {
        Error::new(ErrorKind::Other, err)
    }
}

impl ResponseError for TuserError {
    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code())
            .set_header("Content-Type", "text/html; charset=utf-8")
            .body(format!("{}", self))
    }

    fn status_code(&self) -> StatusCode {
        match self {
            TuserError::FileNotFound => StatusCode::NOT_FOUND,
            TuserError::WrongOffset => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
