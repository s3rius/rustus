use crate::{
    errors::RustusError,
    notifiers::{base::Notifier, hooks::Hook},
    RustusResult,
};
use actix_web::http::header::HeaderMap;
use log::debug;
use tokio::process::Command;

#[derive(Clone)]
pub struct FileNotifier {
    pub command: String,
}

impl FileNotifier {
    pub const fn new(command: String) -> Self {
        Self { command }
    }
}

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
            .arg(message)
            .spawn()?;
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

    use super::FileNotifier;
    use actix_web::http::header::HeaderMap;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::{
        fs::File,
        io::{Read, Write},
    };

    #[cfg(unix)]
    #[actix_rt::test]
    async fn success() {
        let dir = tempdir::TempDir::new("file_notifier").unwrap().into_path();
        let hook_path = dir.join("executable.sh");
        {
            let mut file = File::create(hook_path.clone()).unwrap();
            let mut permissions = file.metadata().unwrap().permissions();
            permissions.set_mode(0o755);
            file.set_permissions(permissions).unwrap();
            let script = r#"#!/bin/sh
            HOOK_NAME="$1";
            MESSAGE="$2";
            echo "$HOOK_NAME $MESSAGE" > "$(dirname $0)/output""#;
            file.write_all(script.as_bytes()).unwrap();
            file.sync_all().unwrap();
        }
        let notifier = FileNotifier::new(hook_path.display().to_string());
        let hook = Hook::PostCreate;
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
        assert_eq!(buffer, format!("{hook} {test_message}\n"));
    }

    #[cfg(unix)]
    #[actix_rt::test]
    async fn error_status() {
        let dir = tempdir::TempDir::new("file_notifier").unwrap().into_path();
        let hook_path = dir.join("error_executable.sh");
        {
            let mut file = File::create(hook_path.clone()).unwrap();
            let mut permissions = file.metadata().unwrap().permissions();
            permissions.set_mode(0o755);
            file.set_permissions(permissions).unwrap();
            let script = r"#!/bin/sh
            read -t 0.1 MESSAGE
            exit 1";
            file.write_all(script.as_bytes()).unwrap();
            file.sync_all().unwrap();
        }
        let notifier = FileNotifier::new(hook_path.display().to_string());
        let res = notifier
            .send_message("test".into(), Hook::PostCreate, &HeaderMap::new())
            .await;
        assert!(res.is_err());
    }

    #[actix_rt::test]
    async fn no_such_file() {
        let notifier = FileNotifier::new(format!("/{}.sh", uuid::Uuid::new_v4()));
        let res = notifier
            .send_message("test".into(), Hook::PreCreate, &HeaderMap::new())
            .await;
        assert!(res.is_err());
    }
}
