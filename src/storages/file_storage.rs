use std::collections::HashMap;
use std::path::PathBuf;

use actix_files::NamedFile;
use async_std::fs::{read_to_string, remove_file, DirBuilder, OpenOptions};
use async_std::prelude::*;
use async_trait::async_trait;
use log::error;
use uuid::Uuid;

use crate::errors::{RustusError, RustusResult};
use crate::storages::{FileInfo, Storage};
use crate::RustusConf;

#[derive(Clone)]
pub struct FileStorage {
    app_conf: RustusConf,
}

impl FileStorage {
    pub fn new(app_conf: RustusConf) -> FileStorage {
        FileStorage { app_conf }
    }

    pub fn info_file_path(&self, file_id: &str) -> PathBuf {
        self.app_conf
            .storage_opts
            .data
            .join(format!("{}.info", file_id))
    }

    pub fn data_file_path(&self, file_id: &str) -> PathBuf {
        self.app_conf.storage_opts.data.join(file_id.to_string())
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        if !self.app_conf.storage_opts.data.exists() {
            DirBuilder::new()
                .create(self.app_conf.storage_opts.data.as_path())
                .await
                .map_err(|err| RustusError::UnableToPrepareStorage(err.to_string()))?;
        }
        Ok(())
    }

    async fn get_file_info(&self, file_id: &str) -> RustusResult<FileInfo> {
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

    async fn set_file_info(&self, file_info: &FileInfo) -> RustusResult<()> {
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

    async fn get_contents(&self, file_id: &str) -> RustusResult<NamedFile> {
        NamedFile::open(self.data_file_path(file_id)).map_err(|err| {
            error!("{:?}", err);
            RustusError::FileNotFound
        })
    }

    async fn add_bytes(
        &self,
        file_id: &str,
        request_offset: usize,
        bytes: &[u8],
    ) -> RustusResult<usize> {
        let mut info = self.get_file_info(file_id).await?;
        if info.offset != request_offset {
            return Err(RustusError::WrongOffset);
        }
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(false)
            .open(self.data_file_path(file_id))
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::UnableToWrite(err.to_string())
            })?;
        file.write_all(bytes).await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToWrite(self.data_file_path(file_id).as_path().display().to_string())
        })?;
        info.offset += bytes.len();
        self.set_file_info(&info).await?;
        Ok(info.offset)
    }

    async fn create_file(
        &self,
        file_size: Option<usize>,
        metadata: Option<HashMap<String, String>>,
    ) -> RustusResult<String> {
        let file_id = Uuid::new_v4().simple().to_string();

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .create_new(true)
            .open(self.data_file_path(file_id.as_str()).as_path())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::FileAlreadyExists(file_id.clone())
            })?;

        // We write empty file here.
        file.write_all(b"").await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToWrite(
                self.data_file_path(file_id.as_str())
                    .as_path()
                    .display()
                    .to_string(),
            )
        })?;

        let file_info = FileInfo::new(
            file_id.as_str(),
            file_size,
            self.data_file_path(file_id.as_str())
                .as_path()
                .display()
                .to_string(),
            metadata,
        );

        self.set_file_info(&file_info).await?;

        Ok(file_id)
    }

    async fn remove_file(&self, file_id: &str) -> RustusResult<()> {
        let info_path = self.info_file_path(file_id);
        if !info_path.exists() {
            return Err(RustusError::FileNotFound);
        }
        let data_path = self.data_file_path(file_id);
        if !data_path.exists() {
            return Err(RustusError::FileNotFound);
        }
        remove_file(info_path).await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToRemove(String::from(file_id))
        })?;
        remove_file(data_path).await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToRemove(String::from(file_id))
        })?;
        Ok(())
    }
}
