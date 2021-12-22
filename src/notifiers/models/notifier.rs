use crate::errors::RustusResult;

use crate::notifiers::Hook;
use async_trait::async_trait;

#[async_trait]
pub trait Notifier {
    async fn prepare(&mut self) -> RustusResult<()>;
    async fn send_message(&self, message: String, hook: Hook) -> RustusResult<()>;
}
