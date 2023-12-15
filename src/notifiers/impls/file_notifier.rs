use crate::{
    errors::RustusError,
    notifiers::{base::Notifier, hooks::Hook},
    RustusResult,
};
use axum::http::HeaderMap;
use tokio::process::Command;

#[derive(Clone, Debug)]
pub struct FileNotifier {
    pub command: String,
}

impl FileNotifier {
    #[must_use]
    pub fn new(command: String) -> Self {
        Self { command }
    }
}

impl Notifier for FileNotifier {
    #[cfg_attr(coverage, no_coverage)]
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    #[tracing::instrument(err, skip(self, message, _headers_map), fields(exit_status = tracing::field::Empty))]
    async fn send_message(
        &self,
        message: &str,
        hook: &Hook,
        _headers_map: &HeaderMap,
    ) -> RustusResult<()> {
        tracing::debug!("Running command: {}", self.command.as_str());
        let mut command = Command::new(self.command.as_str())
            .arg(hook.to_string())
            .arg(message)
            .spawn()?;
        let stat = command.wait().await?;
        if !stat.success() {
            tracing::Span::current().record("exit_status", stat.code().unwrap_or(0));
            return Err(RustusError::HookError("Returned wrong status code".into()));
        }
        Ok(())
    }
}
