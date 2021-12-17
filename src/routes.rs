use actix_web::HttpResponse;

use crate::errors::{RustusError, RustusResult};

/// Default response to all unknown URLs.
#[allow(clippy::unused_async)]
pub async fn not_found() -> RustusResult<HttpResponse> {
    Err(RustusError::FileNotFound)
}
