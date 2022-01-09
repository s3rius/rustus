use std::path::PathBuf;

use actix_files::NamedFile;
use async_std::fs::{remove_file, DirBuilder, OpenOptions};
use async_std::prelude::*;
use async_trait::async_trait;
use log::error;

use crate::errors::{RustusError, RustusResult};
use crate::info_storages::FileInfo;
use crate::storages::Storage;
use crate::RustusConf;
use derive_more::Display;

#[derive(Display)]
#[display(fmt = "file_storage")]
pub struct FileStorage {
    app_conf: RustusConf,
}

impl FileStorage {
    pub fn new(app_conf: RustusConf) -> FileStorage {
        FileStorage { app_conf }
    }

    pub async fn data_file_path(&self, file_id: &str) -> RustusResult<PathBuf> {
        let dir = self
            .app_conf
            .storage_opts
            .data_dir
            // We're working wit absolute paths, because tus.io says so.
            .canonicalize()
            .map_err(|err| {
                error!("{}", err);
                RustusError::UnableToWrite(err.to_string())
            })?
            .join(self.app_conf.dir_struct().as_str());
        DirBuilder::new()
            .recursive(true)
            .create(dir.as_path())
            .await
            .map_err(|err| {
                error!("{}", err);
                RustusError::UnableToWrite(err.to_string())
            })?;
        Ok(dir.join(file_id.to_string()))
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        // We're creating directory for new files
        // if it doesn't already exist.
        if !self.app_conf.storage_opts.data_dir.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(self.app_conf.storage_opts.data_dir.as_path())
                .await
                .map_err(|err| RustusError::UnableToPrepareStorage(err.to_string()))?;
        }
        Ok(())
    }

    async fn get_contents(&self, file_info: &FileInfo) -> RustusResult<NamedFile> {
        if file_info.path.is_none() {
            return Err(RustusError::FileNotFound);
        }
        NamedFile::open_async(file_info.path.clone().unwrap().as_str())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::FileNotFound
            })
    }

    async fn add_bytes(&self, info: &FileInfo, bytes: &[u8]) -> RustusResult<()> {
        // In normal situation this `if` statement is not
        // gonna be called, but what if it is ...
        if info.path.is_none() {
            return Err(RustusError::FileNotFound);
        }
        // Opening file in w+a mode.
        // It means that we're going to append some
        // bytes to the end of a file.
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(false)
            .open(info.path.as_ref().unwrap())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::UnableToWrite(err.to_string())
            })?;
        file.write_all(bytes).await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToWrite(info.path.clone().unwrap())
        })?;
        file.sync_data().await?;
        // Updating information about file.
        Ok(())
    }

    async fn create_file(&self, file_info: &FileInfo) -> RustusResult<String> {
        // New path to file.
        let file_path = self.data_file_path(file_info.id.as_str()).await?;
        // Creating new file.
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .create_new(true)
            .open(file_path.as_path())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::FileAlreadyExists(file_info.id.clone())
            })?;

        // Let's write an empty string to the beginning of the file.
        // Maybe remove it later.
        file.write_all(b"").await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToWrite(file_path.display().to_string())
        })?;
        file.sync_all().await?;

        Ok(file_path.display().to_string())
    }

    async fn remove_file(&self, file_info: &FileInfo) -> RustusResult<()> {
        // Let's remove the file itself.
        let data_path = PathBuf::from(file_info.path.as_ref().unwrap().clone());
        if !data_path.exists() {
            return Err(RustusError::FileNotFound);
        }
        remove_file(data_path).await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToRemove(file_info.id.clone())
        })?;
        Ok(())
    }
}
