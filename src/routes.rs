use actix_web::HttpResponse;

use crate::errors::{RustusError, RustusResult};

/// Default response to all unknown URLs.
/// All protocol urls can be found
/// at `crate::protocol::*`.
#[allow(clippy::unused_async)]
#[cfg_attr(coverage, no_coverage)]
pub async fn not_found() -> RustusResult<HttpResponse> {
    Err(RustusError::FileNotFound)
}
