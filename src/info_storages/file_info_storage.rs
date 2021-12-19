use std::path::PathBuf;

use async_std::fs::{read_to_string, remove_file, DirBuilder, OpenOptions};
use async_std::prelude::*;
use async_trait::async_trait;
use log::error;

use crate::errors::{RustusError, RustusResult};
use crate::info_storages::{FileInfo, InfoStorage};
use crate::RustusConf;

pub struct FileInfoStorage {
    app_conf: RustusConf,
}

impl FileInfoStorage {
    pub fn new(app_conf: RustusConf) -> Self {
        Self { app_conf }
    }

    pub fn info_file_path(&self, file_id: &str) -> PathBuf {
        self.app_conf
            .info_storage_opts
            .info_dir
            .join(format!("{}.info", file_id))
    }
}

#[async_trait]
impl InfoStorage for FileInfoStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        if !self.app_conf.info_storage_opts.info_dir.exists() {
            DirBuilder::new()
                .create(self.app_conf.info_storage_opts.info_dir.as_path())
                .await
                .map_err(|err| RustusError::UnableToPrepareInfoStorage(err.to_string()))?;
        }
        Ok(())
    }

    async fn set_info(&self, file_info: &FileInfo) -> RustusResult<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(self.info_file_path(file_info.id.as_str()).as_path())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::UnableToWrite(err.to_string())
            })?;
        file.write_all(serde_json::to_string(&file_info)?.as_bytes())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::UnableToWrite(
                    self.info_file_path(file_info.id.as_str())
                        .as_path()
                        .display()
                        .to_string(),
                )
            })?;
        Ok(())
    }

    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo> {
        let info_path = self.info_file_path(file_id);
        if !info_path.exists() {
            return Err(RustusError::FileNotFound);
        }
        let contents = read_to_string(info_path).await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToReadInfo
        })?;
        serde_json::from_str::<FileInfo>(contents.as_str()).map_err(RustusError::from)
    }

    async fn remove_info(&self, file_id: &str) -> RustusResult<()> {
        let info_path = self.info_file_path(file_id);
        if !info_path.exists() {
            return Err(RustusError::FileNotFound);
        }
        remove_file(info_path).await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToRemove(String::from(file_id))
        })
    }
}
