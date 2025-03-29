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
    #[error("Unable to configure logging: {0}")]
    LogConfigError(#[from] log::SetLoggerError),
    #[error("Kafka extra options error: {0}")]
    KafkaExtraOptionsError(String),
    #[error("AMQP error: {0}")]
    AMQPError(#[from] lapin::Error),
    #[error("AMQP pooling error: {0}")]
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
    #[error("Metrics error: {0}")]
    PrometheusError(#[from] prometheus::Error),
    #[error("Blocking error: {0}")]
    BlockingError(#[from] actix_web::error::BlockingError),
    #[error("HTTP hook error. Returned status: {0}, Response text: {1}")]
    HTTPHookError(u16, String, Option<String>),
    #[error("Found S3 error: {0}")]
    S3Error(#[from] s3::error::S3Error),
    #[error("Missing S3 upload id in metadata")]
    S3UploadIdMissing,
    #[error("Can't parse integer: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Can't convert int: {0}")]
    TryFromIntError(#[from] std::num::TryFromIntError),
    #[error("Kafka error: {0}")]
    KafkaError(#[from] rdkafka::error::KafkaError),
    #[error("Nats connection error: {0}")]
    NatsConnectError(#[from] async_nats::ConnectError),
    #[error("Nats publish error: {0}")]
    NatsPublishError(#[from] async_nats::PublishError),
    #[error("Nats request error: {0}")]
    NatsRequestError(#[from] async_nats::RequestError),
    #[error("Received error response from NATS: {0}")]
    NatsErrorResponse(String),
    #[error("Nkeys error: {0}")]
    NkeysError(#[from] nkeys::error::Error),
}

/// This conversion allows us to use `RustusError` in the `main` function.
impl From<RustusError> for Error {
    fn from(err: RustusError) -> Self {
        Self::new(ErrorKind::Other, err)
    }
}

/// Trait to convert errors to http-responses.
impl ResponseError for RustusError {
    fn error_response(&self) -> HttpResponse {
        error!("{}", self);
        match self {
            Self::HTTPHookError(_, proxy_response, content_type) => {
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
            Self::FileNotFound => StatusCode::NOT_FOUND,
            Self::WrongOffset => StatusCode::CONFLICT,
            Self::FrozenFile
            | Self::SizeAlreadyKnown
            | Self::HookError(_)
            | Self::UnknownHashAlgorithm
            | Self::WrongHeaderValue => StatusCode::BAD_REQUEST,
            Self::WrongChecksum => StatusCode::EXPECTATION_FAILED,
            Self::HTTPHookError(status, _, _) => {
                StatusCode::from_u16(*status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
