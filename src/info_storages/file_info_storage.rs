use std::{
    io::{Read, Write},
    path::PathBuf,
};

use async_trait::async_trait;
use log::error;
use std::{
    fs::{remove_file, File, OpenOptions},
    io::{BufReader, BufWriter},
};
use tokio::fs::DirBuilder;

use crate::{
    errors::{RustusError, RustusResult},
    info_storages::{FileInfo, InfoStorage},
};

#[derive(Clone)]
pub struct FileInfoStorage {
    info_dir: PathBuf,
}

impl FileInfoStorage {
    pub fn new(info_dir: PathBuf) -> Self {
        Self { info_dir }
    }

    pub fn info_file_path(&self, file_id: &str) -> PathBuf {
        self.info_dir.join(format!("{}.info", file_id))
    }
}

#[async_trait(?Send)]
impl InfoStorage for FileInfoStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        if !self.info_dir.exists() {
            DirBuilder::new()
                .create(self.info_dir.as_path())
                .await
                .map_err(|err| RustusError::UnableToPrepareInfoStorage(err.to_string()))?;
        }
        Ok(())
    }

    async fn set_info(&self, file_info: &FileInfo, create: bool) -> RustusResult<()> {
        let info = file_info.clone();
        let path = self.info_file_path(info.id.as_str());
        tokio::task::spawn_blocking(move || {
            let file = OpenOptions::new()
                .write(true)
                .create(create)
                .truncate(true)
                .open(path)
                .map_err(|err| {
                    error!("{:?}", err);
                    RustusError::UnableToWrite(err.to_string())
                })?;
            let data = serde_json::to_string(&info).map_err(RustusError::from)?;
            {
                let mut writer = BufWriter::new(file);
                writer.write_all(data.as_bytes())?;
                writer.flush()?;
            }
            Ok(())
        })
        .await?
    }

    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo> {
        let info_path = self.info_file_path(file_id);
        tokio::task::spawn_blocking(move || {
            if !info_path.exists() {
                return Err(RustusError::FileNotFound);
            }
            let info = File::open(info_path)?;
            let mut contents = String::new();
            let mut reader = BufReader::new(info);
            reader.read_to_string(&mut contents)?;
            serde_json::from_str::<FileInfo>(contents.as_str()).map_err(RustusError::from)
        })
        .await?
    }

    async fn remove_info(&self, file_id: &str) -> RustusResult<()> {
        let id = String::from(file_id);
        let info_path = self.info_file_path(id.as_str());
        tokio::task::spawn_blocking(move || {
            if !info_path.exists() {
                return Err(RustusError::FileNotFound);
            }
            remove_file(info_path).map_err(|err| {
                error!("{:?}", err);
                RustusError::UnableToRemove(id)
            })
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::FileInfoStorage;
    use crate::{info_storages::FileInfo, InfoStorage};
    use std::{
        collections::HashMap,
        fs::File,
        io::{Read, Write},
    };

    #[actix_rt::test]
    async fn preparation() {
        let dir = tempdir::TempDir::new("file_info").unwrap();
        let target_path = dir.into_path().join("not_exist");
        let mut storage = FileInfoStorage::new(target_path.clone());
        assert!(!target_path.exists());
        storage.prepare().await.unwrap();
        assert!(target_path.exists());
    }

    #[actix_rt::test]
    async fn setting_info() {
        let dir = tempdir::TempDir::new("file_info").unwrap();
        let storage = FileInfoStorage::new(dir.into_path());
        let file_info = FileInfo::new(
            uuid::Uuid::new_v4().to_string().as_str(),
            Some(10),
            Some("random_path".into()),
            "random_storage".into(),
            None,
        );
        storage.set_info(&file_info, true).await.unwrap();
        let info_path = storage.info_file_path(file_info.id.as_str());
        let mut buffer = String::new();
        File::open(info_path)
            .unwrap()
            .read_to_string(&mut buffer)
            .unwrap();
        assert!(buffer.len() > 0);
    }

    #[actix_rt::test]
    async fn set_get_info() {
        let dir = tempdir::TempDir::new("file_info").unwrap();
        let storage = FileInfoStorage::new(dir.into_path());
        let file_info = FileInfo::new(
            uuid::Uuid::new_v4().to_string().as_str(),
            Some(10),
            Some("random_path".into()),
            "random_storage".into(),
            {
                let mut a = HashMap::new();
                a.insert("test".into(), "pest".into());
                Some(a)
            },
        );
        storage.set_info(&file_info, true).await.unwrap();
        let read_info = storage.get_info(file_info.id.as_str()).await.unwrap();
        assert_eq!(read_info.id, read_info.id);
        assert_eq!(read_info.length, read_info.length);
        assert_eq!(read_info.path, read_info.path);
        assert_eq!(read_info.metadata, read_info.metadata);
    }

    #[actix_rt::test]
    async fn get_broken_info() {
        let dir = tempdir::TempDir::new("file_info").unwrap();
        let storage = FileInfoStorage::new(dir.into_path());
        let file_id = "random_file";
        let mut file = File::create(storage.info_file_path(file_id)).unwrap();
        file.write_all("{not a json}".as_bytes()).unwrap();
        let read_info = storage.get_info(file_id).await;
        assert!(read_info.is_err());
    }
}
