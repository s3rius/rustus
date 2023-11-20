use crate::{
    errors::RustusError,
    notifiers::{base::Notifier, hooks::Hook},
    RustusResult,
};
use axum::http::HeaderMap;
use log::debug;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Clone)]
pub struct DirNotifier {
    pub dir: PathBuf,
}

impl DirNotifier {
    #[must_use]
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

impl Notifier for DirNotifier {
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
        let hook_path = self.dir.join(hook.to_string());
        if !hook_path.exists() {
            debug!("Hook {} not found.", hook.to_string());
            return Err(RustusError::HookError(format!(
                "Hook file {hook} not found."
            )));
        }
        debug!("Running hook: {}", hook_path.as_path().display());
        let mut command = Command::new(hook_path).arg(message).spawn()?;
        let stat = command.wait().await?;
        if !stat.success() {
            return Err(RustusError::HookError("Returned wrong status code".into()));
        }
        Ok(())
    }
}
