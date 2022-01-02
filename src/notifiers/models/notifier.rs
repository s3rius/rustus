use crate::errors::RustusResult;
use actix_web::http::header::HeaderMap;

use crate::notifiers::Hook;
use async_trait::async_trait;

#[async_trait]
pub trait Notifier {
    async fn prepare(&mut self) -> RustusResult<()>;
    async fn send_message(
        &self,
        message: String,
        hook: Hook,
        headers_map: &HeaderMap,
    ) -> RustusResult<()>;
}
