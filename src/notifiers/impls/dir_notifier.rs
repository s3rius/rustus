use crate::{
    errors::RustusError,
    notifiers::{base::Notifier, hooks::Hook},
    RustusResult,
};
use actix_web::http::header::HeaderMap;
use log::debug;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Clone)]
pub struct DirNotifier {
    pub dir: PathBuf,
}

impl DirNotifier {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

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

#[cfg(test)]
mod tests {
    use crate::notifiers::{base::Notifier, hooks::Hook};

    use super::DirNotifier;
    use actix_web::http::header::HeaderMap;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::{
        fs::File,
        io::{Read, Write},
    };
    use tempdir::TempDir;

    #[actix_rt::test]
    async fn no_such_hook_file() {
        let hook_dir = TempDir::new("dir_notifier").unwrap().into_path();
        let notifier = DirNotifier::new(hook_dir);
        let res = notifier
            .send_message("test".into(), Hook::PostCreate, &HeaderMap::new())
            .await;
        assert!(res.is_err());
    }

    #[cfg(unix)]
    #[actix_rt::test]
    async fn success() {
        let hook = Hook::PostCreate;
        let dir = tempdir::TempDir::new("dir_notifier").unwrap().into_path();
        let hook_path = dir.join(hook.to_string());
        {
            let mut file = File::create(hook_path.clone()).unwrap();
            let mut permissions = file.metadata().unwrap().permissions();
            permissions.set_mode(0o755);
            file.set_permissions(permissions).unwrap();
            let script = r#"#!/bin/sh
            echo "$1" > "$(dirname $0)/output""#;
            file.write_all(script.as_bytes()).unwrap();
            file.sync_all().unwrap();
        }
        let notifier = DirNotifier::new(dir.to_path_buf());
        let test_message = uuid::Uuid::new_v4().to_string();
        notifier
            .send_message(test_message.clone(), hook, &HeaderMap::new())
            .await
            .unwrap();
        let output_path = dir.join("output");
        assert!(output_path.exists());
        let mut buffer = String::new();
        let mut out_file = File::open(output_path).unwrap();
        out_file.read_to_string(&mut buffer).unwrap();
        assert_eq!(buffer, format!("{}\n", test_message));
    }
}
