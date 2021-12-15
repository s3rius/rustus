use std::collections::HashMap;

use actix_files::NamedFile;
use async_std::fs::{DirBuilder, OpenOptions, read_to_string};
use async_std::prelude::*;
use async_trait::async_trait;
use log::error;
use uuid::Uuid;

use crate::errors::{TuserError, TuserResult};
use crate::storages::{FileInfo, Storage};
use crate::TuserConf;

#[derive(Clone)]
pub struct FileStorage {
    app_conf: TuserConf,
}

impl FileStorage {
    pub fn new(app_conf: TuserConf) -> FileStorage {
        FileStorage { app_conf }
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn prepare(&self) -> TuserResult<()> {
        if !self.app_conf.data.exists() {
            DirBuilder::new()
                .create(self.app_conf.data.as_path())
                .await
                .map_err(|err| TuserError::UnableToPrepareStorage(err.to_string()))?;
        }
        Ok(())
    }

    async fn get_file_info(&self, file_id: &str) -> TuserResult<FileInfo> {
        let info_file_path = self.app_conf.data.join(format!("{}.info", file_id));
        let contents = read_to_string(info_file_path).await.map_err(|err| {
            error!("{:?}", err);
            TuserError::UnableToReadInfo
        })?;
        serde_json::from_str::<FileInfo>(contents.as_str()).map_err(TuserError::from)
    }

    async fn set_file_info(&self, file_info: &FileInfo) -> TuserResult<()> {
        let info_file_path = self.app_conf.data.join(format!("{}.info", file_info.id));
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(info_file_path.as_path())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                TuserError::UnableToWrite(err.to_string())
            })?;
        file.write_all(serde_json::to_string(&file_info)?.as_bytes())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                TuserError::UnableToWrite(info_file_path.as_path().display().to_string())
            })?;
        Ok(())
    }

    async fn get_contents(&self, file_id: &str) -> TuserResult<NamedFile> {
        Err(TuserError::FileNotFound(String::from(file_id)))
    }

    async fn add_bytes(&self, file_id: &str, request_offset: usize, bytes: &[u8]) -> TuserResult<usize> {
        let file_path = self.app_conf.data.join(file_id);
        let mut info = self.get_file_info(file_id).await?;
        if info.offset != request_offset {
            return Err(TuserError::WrongOffset);
        }
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(false)
            .open(file_path.as_path())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                TuserError::UnableToWrite(err.to_string())
            })?;
        file.write_all(bytes).await.map_err(|err| {
            error!("{:?}", err);
            TuserError::UnableToWrite(file_path.as_path().display().to_string())
        })?;
        info.offset += bytes.len();
        self.set_file_info(&info).await?;
        Ok(info.offset)
    }

    async fn create_file(
        &self,
        file_size: Option<usize>,
        metadata: Option<HashMap<String, String>>,
    ) -> TuserResult<String> {
        let file_id = Uuid::new_v4().simple().to_string();
        let file_path = self.app_conf.data.join(file_id.as_str());
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .create_new(true)
            .open(file_path.as_path())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                TuserError::FileAlreadyExists(file_id.clone())
            })?;

        // We write empty file here.
        file.write_all(b"").await.map_err(|err| {
            error!("{:?}", err);
            TuserError::UnableToWrite(file_path.as_path().display().to_string())
        })?;

        let file_info = FileInfo::new(
            file_id.as_str(),
            file_size,
            file_path.as_path().display().to_string(),
            metadata,
        );

        self.set_file_info(&file_info).await?;

        Ok(file_id)
    }
}
