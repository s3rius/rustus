use crate::errors::RustusError;
use crate::notifiers::{Hook, Notifier};
use crate::RustusResult;
use actix_web::http::header::HeaderMap;
use async_process::{Command, Stdio};
use async_trait::async_trait;
use futures::AsyncWriteExt;
use log::debug;

pub struct FileNotifier {
    pub command: String,
}

impl FileNotifier {
    pub fn new(command: String) -> Self {
        Self { command }
    }
}

#[async_trait]
impl Notifier for FileNotifier {
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
            .stdin(Stdio::piped())
            .spawn()?;
        command
            .stdin
            .as_mut()
            .unwrap()
            .write_all(message.as_bytes())
            .await?;
        let stat = command.status().await?;
        if !stat.success() {
            return Err(RustusError::HookError("Returned wrong status code".into()));
        }
        Ok(())
    }
}
