use axum::http::HeaderMap;

use crate::errors::RustusResult;

use super::hooks::Hook;

pub trait Notifier {
    async fn prepare(&mut self) -> RustusResult<()>;
    async fn send_message(
        &self,
        message: &str,
        hook: &Hook,
        headers_map: &HeaderMap,
    ) -> RustusResult<()>;
}
