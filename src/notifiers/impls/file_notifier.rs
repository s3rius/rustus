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
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    #[tracing::instrument(
        err,
        skip(self, message, _headers_map),
        fields(
            exit_status = tracing::field::Empty,
            sout = tracing::field::Empty,
            serr = tracing::field::Empty,
        )
    )]
    async fn send_message(
        &self,
        message: &str,
        hook: &Hook,
        _headers_map: &HeaderMap,
    ) -> RustusResult<()> {
        let hook_str = hook.to_string();
        tracing::info!(
            "Running command: `{} \"{}\" \"{{message}}\"`",
            self.command.as_str(),
            &hook_str
        );

        let command = Command::new(self.command.as_str())
            .arg(hook_str)
            .arg(message)
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        let output = command.wait_with_output().await?;

        tracing::Span::current()
            .record("exit_status", output.status.code().unwrap_or(0))
            .record("sout", String::from_utf8_lossy(&output.stdout).to_string())
            .record("serr", String::from_utf8_lossy(&output.stderr).to_string());

        if !output.status.success() {
            return Err(RustusError::HookError(String::from(
                "Returned wrong status code",
            )));
        }

        Ok(())
    }
}
