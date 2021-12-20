use std::collections::HashMap;
use std::path::PathBuf;

use actix_files::NamedFile;
use async_std::fs::create_dir_all;
use async_std::fs::{remove_file, DirBuilder, OpenOptions};
use async_std::prelude::*;
use async_trait::async_trait;
use log::error;
use uuid::Uuid;

use crate::errors::{RustusError, RustusResult};
use crate::info_storages::{FileInfo, InfoStorage};
use crate::storages::Storage;
use crate::RustusConf;

pub struct FileStorage {
    app_conf: RustusConf,
    info_storage: Box<dyn InfoStorage + Send + Sync>,
}

impl FileStorage {
    pub fn new(
        app_conf: RustusConf,
        info_storage: Box<dyn InfoStorage + Send + Sync>,
    ) -> FileStorage {
        FileStorage {
            app_conf,
            info_storage,
        }
    }

    pub async fn data_file_path(&self, file_id: &str) -> RustusResult<PathBuf> {
        let dir = self
            .app_conf
            .storage_opts
            .data_dir
            .join(self.app_conf.dir_struct());
        create_dir_all(dir.as_path()).await.map_err(|err| {
            error!("{}", err);
            RustusError::UnableToWrite(err.to_string())
        })?;
        Ok(dir.join(file_id.to_string()))
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        if !self.app_conf.storage_opts.data_dir.exists() {
            DirBuilder::new()
                .create(self.app_conf.storage_opts.data_dir.as_path())
                .await
                .map_err(|err| RustusError::UnableToPrepareStorage(err.to_string()))?;
        }
        Ok(())
    }

    async fn get_file_info(&self, file_id: &str) -> RustusResult<FileInfo> {
        self.info_storage.get_info(file_id).await
    }

    async fn get_contents(&self, file_id: &str) -> RustusResult<NamedFile> {
        let info = self.info_storage.get_info(file_id).await?;
        NamedFile::open(info.path.as_str()).map_err(|err| {
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
        let mut info = self.info_storage.get_info(file_id).await?;
        if info.offset != request_offset {
            return Err(RustusError::WrongOffset);
        }
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(false)
            .open(info.path.as_str())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::UnableToWrite(err.to_string())
            })?;
        file.write_all(bytes).await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToWrite(info.path.clone())
        })?;
        info.offset += bytes.len();
        self.info_storage.set_info(&info, false).await?;
        Ok(info.offset)
    }

    async fn create_file(
        &self,
        file_size: Option<usize>,
        metadata: Option<HashMap<String, String>>,
    ) -> RustusResult<String> {
        let file_id = Uuid::new_v4().simple().to_string();
        let file_path = self.data_file_path(file_id.as_str()).await?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .create_new(true)
            .open(file_path.as_path())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::FileAlreadyExists(file_id.clone())
            })?;

        // We write empty file here.
        file.write_all(b"").await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToWrite(file_path.display().to_string())
        })?;

        let file_info = FileInfo::new(
            file_id.as_str(),
            file_size,
            file_path.display().to_string(),
            metadata,
        );

        self.info_storage.set_info(&file_info, true).await?;

        Ok(file_id)
    }

    async fn remove_file(&self, file_id: &str) -> RustusResult<()> {
        let info = self.info_storage.get_info(file_id).await?;
        self.info_storage.remove_info(file_id).await?;

        let data_path = PathBuf::from(info.path.clone());
        if !data_path.exists() {
            return Err(RustusError::FileNotFound);
        }
        remove_file(data_path).await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToRemove(String::from(file_id))
        })?;
        Ok(())
    }
}
