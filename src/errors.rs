use std::io::{Error, ErrorKind};

use actix_web::{HttpResponse, ResponseError};
use actix_web::dev::HttpResponseBuilder;
use actix_web::http::StatusCode;
use thiserror::Error;

pub type TuserResult<T> = Result<T, TuserError>;

#[derive(Error, Debug)]
pub enum TuserError {
    #[error("File with id {0} was not found")]
    FileNotFound(String),
    #[error("File with id {0} already exists")]
    FileAlreadyExists(String),
    #[error("Given offset is incorrect.")]
    WrongOffset,
    #[error("Unknown error")]
    Unknown,
    #[error("Unable to serialize object")]
    UnableToSerialize(#[from] serde_json::Error),
    #[error("Unable to get file information")]
    UnableToReadInfo,
    #[error("Unable to write file {0}")]
    UnableToWrite(String),
    #[error("Unable to prepare storage. Reason: {0}")]
    UnableToPrepareStorage(String),
}

impl From<TuserError> for Error {
    fn from(err: TuserError) -> Self {
        Error::new(ErrorKind::Other, err)
    }
}

impl ResponseError for TuserError {
    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code()).body(format!("{}", self))
    }

    fn status_code(&self) -> StatusCode {
        match self {
            TuserError::FileNotFound(_) => StatusCode::NOT_FOUND,
            TuserError::WrongOffset => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
