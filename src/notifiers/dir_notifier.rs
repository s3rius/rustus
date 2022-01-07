use crate::errors::RustusError;
use crate::notifiers::{Hook, Notifier};
use crate::RustusResult;
use actix_web::http::header::HeaderMap;
use async_process::{Command, Stdio};
use async_trait::async_trait;
use futures::AsyncWriteExt;
use log::debug;
use std::path::PathBuf;

pub struct DirNotifier {
    pub dir: PathBuf,
}

impl DirNotifier {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

#[async_trait]
impl Notifier for DirNotifier {
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
        debug!("Running hook: {}", hook_path.as_path().display());
        let mut command = Command::new(hook_path).stdin(Stdio::piped()).spawn()?;
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
