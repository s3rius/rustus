use crate::{
    errors::RustusError,
    notifiers::{base::Notifier, hooks::Hook},
    RustusResult,
};
use axum::http::HeaderMap;
use log::debug;
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

    async fn send_message(
        &self,
        message: String,
        hook: Hook,
        _headers_map: &HeaderMap,
    ) -> RustusResult<()> {
        debug!("Running command: {}", self.command.as_str());
        let mut command = Command::new(self.command.as_str())
            .arg(hook.to_string())
            .arg(message)
            .spawn()?;
        let stat = command.wait().await?;
        if !stat.success() {
            return Err(RustusError::HookError("Returned wrong status code".into()));
        }
        Ok(())
    }
}
