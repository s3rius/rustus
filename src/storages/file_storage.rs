use std::collections::HashMap;
use std::path::PathBuf;

use actix_files::NamedFile;
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

    async fn get_file_info(&self, file_id: &str) -> RustusResult<FileInfo> {
        // I don't think comments are convenient here.
        self.info_storage.get_info(file_id).await
    }

    async fn get_contents(&self, file_id: &str) -> RustusResult<NamedFile> {
        let info = self.info_storage.get_info(file_id).await?;
        if info.path.is_none() {
            return Err(RustusError::FileNotFound);
        }
        NamedFile::open_async(info.path.unwrap().as_str())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::FileNotFound
            })
    }

    async fn add_bytes(
        &self,
        file_id: &str,
        request_offset: usize,
        updated_length: Option<usize>,
        bytes: &[u8],
    ) -> RustusResult<FileInfo> {
        let mut info = self.info_storage.get_info(file_id).await?;
        // Checking that provided offset is equal to offset provided by request.
        if info.offset != request_offset {
            return Err(RustusError::WrongOffset);
        }
        // In normal situation this `if` statement is not
        // gonna be called, but what if it is ...
        if info.path.is_none() {
            return Err(RustusError::FileNotFound);
        }
        // This thing is only applicable in case
        // if tus-extension `creation-defer-length` is enabled.
        if let Some(new_len) = updated_length {
            // Whoop, someone gave us total file length
            // less that he had already uploaded.
            if new_len < info.offset {
                return Err(RustusError::WrongOffset);
            }
            // We already know the exact size of a file.
            // Someone want to update it.
            // Anyway, it's not allowed, heh.
            if info.length.is_some() {
                return Err(RustusError::SizeAlreadyKnown);
            }

            // All checks are ok. Now our file will have exact size.
            info.deferred_size = false;
            info.length = Some(new_len);
        }
        // Checking if the size of the upload is already equals
        // to calculated offset. It means that all bytes were already written.
        if Some(info.offset) == info.length {
            return Err(RustusError::FrozenFile);
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
        info.offset += bytes.len();
        self.info_storage.set_info(&info, false).await?;
        Ok(info)
    }

    async fn create_file(
        &self,
        file_size: Option<usize>,
        metadata: Option<HashMap<String, String>>,
    ) -> RustusResult<FileInfo> {
        // Let's create a new file ID.
        // I guess the algo for generating new upload-id's can be
        // configurable. But for now I don't really care, since UUIv4 works fine.
        // Maybe update it later.
        let file_id = Uuid::new_v4().simple().to_string();
        // New path to file.
        let file_path = self.data_file_path(file_id.as_str()).await?;
        // Creating new file.
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

        // Let's write an empty string to the beginning of the file.
        // Maybe remove it later.
        file.write_all(b"").await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToWrite(file_path.display().to_string())
        })?;
        file.sync_all().await?;
        // Creating new FileInfo object and saving it.
        let file_info = FileInfo::new(
            file_id.as_str(),
            file_size,
            Some(file_path.display().to_string()),
            metadata,
        );

        self.info_storage.set_info(&file_info, true).await?;

        Ok(file_info)
    }

    async fn remove_file(&self, file_id: &str) -> RustusResult<FileInfo> {
        let info = self.info_storage.get_info(file_id).await?;
        // Whoops, someone forgot to update the path field.
        if info.path.is_none() {
            return Err(RustusError::FileNotFound);
        }
        // Let's remove info first, so file won't show up
        // In get_contents function.
        self.info_storage.remove_info(file_id).await?;

        // Let's remove the file itself.
        let data_path = PathBuf::from(info.path.as_ref().unwrap().clone());
        if !data_path.exists() {
            // Maybe we don't need error here,
            // since if file doesn't exist,  we're done.
            // FIXME: Find it out.
            return Err(RustusError::FileNotFound);
        }
        remove_file(data_path).await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToRemove(String::from(file_id))
        })?;
        Ok(info)
    }
}
