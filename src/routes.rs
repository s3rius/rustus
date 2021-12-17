use actix_web::HttpResponse;

use crate::errors::{TuserError, TuserResult};

/// Default response to all unknown URLs.
#[allow(clippy::unused_async)]
pub async fn not_found() -> TuserResult<HttpResponse> {
    Err(TuserError::FileNotFound)
}
