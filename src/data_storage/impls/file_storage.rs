use std::{fs::File, io::Write, path::PathBuf};

use actix_files::NamedFile;
use actix_web::{HttpRequest, HttpResponse};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use log::error;
use std::{
    fs::{remove_file, DirBuilder, OpenOptions},
    io::{copy, BufReader, BufWriter},
};

use crate::{
    data_storage::base::DataStorage,
    errors::{RustusError, RustusResult},
    file_info::FileInfo,
    utils::dir_struct::substr_time,
};
use derive_more::Display;

#[derive(Display, Clone, Debug)]
#[display("file_storage")]
pub struct FileDataStorage {
    data_dir: PathBuf,
    dir_struct: String,
    force_fsync: bool,
}

impl FileDataStorage {
    pub const fn new(data_dir: PathBuf, dir_struct: String, force_fsync: bool) -> Self {
        Self {
            data_dir,
            dir_struct,
            force_fsync,
        }
    }

    pub fn data_file_path(
        &self,
        file_id: &str,
        created_at: DateTime<Utc>,
    ) -> RustusResult<PathBuf> {
        let dir = self
            .data_dir
            // We're working wit absolute paths, because tus.io says so.
            .canonicalize()
            .map_err(|err| {
                error!("{}", err);
                RustusError::UnableToWrite(err.to_string())
            })?
            .join(substr_time(self.dir_struct.as_str(), created_at));
        DirBuilder::new()
            .recursive(true)
            .create(dir.as_path())
            .map_err(|err| {
                error!("{}", err);
                RustusError::UnableToWrite(err.to_string())
            })?;
        Ok(dir.join(file_id))
    }
}

impl DataStorage for FileDataStorage {
    fn get_name(&self) -> &'static str {
        "file_storage"
    }

    async fn prepare(&mut self) -> RustusResult<()> {
        // We're creating directory for new files
        // if it doesn't already exist.
        if !self.data_dir.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(self.data_dir.as_path())
                .map_err(|err| RustusError::UnableToPrepareStorage(err.to_string()))?;
        }
        Ok(())
    }

    async fn get_contents(
        &self,
        file_info: &FileInfo,
        request: &HttpRequest,
    ) -> RustusResult<HttpResponse> {
        if let Some(path) = &file_info.path {
            let file = File::open(path).map_err(|err| {
                error!("{:?}", err);
                RustusError::FileNotFound
            })?;
            Ok(NamedFile::from_file(file, file_info.get_filename())
                .map_err(|err| {
                    error!("{:?}", err);
                    RustusError::FileNotFound
                })?
                .into_response(request))
        } else {
            Err(RustusError::FileNotFound)
        }
    }

    async fn add_bytes(&self, file_info: &FileInfo, mut bytes: Bytes) -> RustusResult<()> {
        // In normal situation this `if` statement is not
        // gonna be called, but what if it is ...
        if file_info.path.is_none() {
            return Err(RustusError::FileNotFound);
        }
        let path = self.data_file_path(&file_info.id, file_info.created_at)?;
        let force_sync = self.force_fsync;
        tokio::task::spawn_blocking(move || {
            // Opening file in w+a mode.
            // It means that we're going to append some
            // bytes to the end of a file.
            let file = OpenOptions::new()
                .append(true)
                .create(false)
                .read(false)
                .truncate(false)
                .open(path)
                .map_err(|err| {
                    error!("{:?}", err);
                    RustusError::UnableToWrite(err.to_string())
                })?;
            let mut writer = BufWriter::new(file);
            writer.write_all(bytes.as_ref())?;
            writer.flush()?;
            if force_sync {
                writer.get_ref().sync_data()?;
            }
            bytes.clear();

            Ok(())
        })
        .await?
    }

    async fn create_file(&self, file_info: &FileInfo) -> RustusResult<String> {
        // New path to file.
        let file_path = self.data_file_path(file_info.id.as_str(), file_info.created_at)?;
        tokio::task::spawn_blocking(move || {
            // Creating new file.
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .create_new(true)
                .open(file_path.as_path())
                .map_err(|err| {
                    error!("{:?}", err);
                    RustusError::FileAlreadyExists
                })?;
            Ok(file_path.display().to_string())
        })
        .await?
    }

    async fn concat_files(
        &self,
        file_info: &FileInfo,
        parts_info: Vec<FileInfo>,
    ) -> RustusResult<()> {
        let force_fsync = self.force_fsync;
        let path = self.data_file_path(&file_info.id, file_info.created_at)?;
        let part_paths = parts_info
            .iter()
            .map(|info| self.data_file_path(&info.id, info.created_at))
            .collect::<Result<Vec<PathBuf>, _>>()?;
        tokio::task::spawn_blocking(move || {
            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(path)
                .map_err(|err| {
                    error!("{:?}", err);
                    RustusError::UnableToWrite(err.to_string())
                })?;
            let mut writer = BufWriter::new(file);
            for part in part_paths {
                let part_file = OpenOptions::new().read(true).open(part)?;
                let mut reader = BufReader::new(part_file);
                copy(&mut reader, &mut writer)?;
            }
            writer.flush()?;
            if force_fsync {
                writer.get_ref().sync_data()?;
            }
            Ok(())
        })
        .await?
    }

    async fn remove_file(&self, file_info: &FileInfo) -> RustusResult<()> {
        let info = file_info.clone();
        let file_path = self.data_file_path(&info.id, info.created_at)?;
        tokio::task::spawn_blocking(move || {
            // Let's remove the file itself.
            if !file_path.exists() {
                return Err(RustusError::FileNotFound);
            }
            remove_file(file_path).map_err(|err| {
                error!("{:?}", err);
                RustusError::UnableToRemove(info.id.clone())
            })?;
            Ok(())
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::FileDataStorage;
    use crate::{data_storage::base::DataStorage, file_info::FileInfo};
    use actix_web::test::TestRequest;
    use bytes::Bytes;
    use std::{
        fs::File,
        io::{Read, Write},
        path::PathBuf,
    };

    #[actix_rt::test]
    async fn preparation() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let target_path = dir.into_path().join("not_exist");
        let mut storage = FileDataStorage::new(target_path.clone(), String::new(), false);
        assert!(!target_path.exists());
        storage.prepare().await.unwrap();
        assert!(target_path.exists());
    }

    #[actix_rt::test]
    async fn create_file() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileDataStorage::new(dir.into_path(), String::new(), false);
        let file_info = FileInfo::new("test_id", Some(5), None, storage.to_string(), None);
        let new_path = storage.create_file(&file_info).await.unwrap();
        assert!(PathBuf::from(new_path).exists());
    }

    #[actix_rt::test]
    async fn create_file_but_it_exists() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let base_path = dir.into_path().clone();
        let storage = FileDataStorage::new(base_path.clone(), String::new(), false);
        let file_info = FileInfo::new("test_id", Some(5), None, storage.to_string(), None);
        File::create(base_path.join("test_id")).unwrap();
        let result = storage.create_file(&file_info).await;
        assert!(result.is_err());
    }

    #[actix_rt::test]
    async fn adding_bytes() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileDataStorage::new(dir.into_path(), String::new(), false);
        let mut file_info = FileInfo::new("test_id", Some(5), None, storage.to_string(), None);
        let new_path = storage.create_file(&file_info).await.unwrap();
        let test_data = "MyTestData";
        file_info.path = Some(new_path.clone());
        storage
            .add_bytes(&file_info, Bytes::from(test_data))
            .await
            .unwrap();
        let mut file = File::open(new_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, String::from(test_data));
    }

    #[actix_rt::test]
    async fn adding_bytes_to_unknown_file() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileDataStorage::new(dir.into_path(), String::new(), false);
        let file_info = FileInfo::new(
            "test_id",
            Some(5),
            Some(String::from("some_file")),
            storage.to_string(),
            None,
        );
        let test_data = "MyTestData";
        let result = storage.add_bytes(&file_info, Bytes::from(test_data)).await;
        assert!(result.is_err());
    }

    #[actix_rt::test]
    async fn get_contents_of_unknown_file() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileDataStorage::new(dir.into_path(), String::new(), false);
        let file_info = FileInfo::new(
            "test_id",
            Some(5),
            Some(storage.data_dir.join("unknown").display().to_string()),
            storage.to_string(),
            None,
        );
        let request = TestRequest::get().to_http_request();
        let file_info = storage.get_contents(&file_info, &request).await;
        assert!(file_info.is_err());
    }

    #[actix_rt::test]
    async fn remove_unknown_file() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileDataStorage::new(dir.into_path(), String::new(), false);
        let file_info = FileInfo::new(
            "test_id",
            Some(5),
            Some(storage.data_dir.join("unknown").display().to_string()),
            storage.to_string(),
            None,
        );
        let file_info = storage.remove_file(&file_info).await;
        assert!(file_info.is_err());
    }

    #[actix_rt::test]
    async fn success_concatenation() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileDataStorage::new(dir.into_path(), String::new(), false);

        let mut parts = Vec::new();
        let mut part1 = FileInfo::new("part_id1", None, None, storage.to_string(), None);
        part1.path = Some(
            storage
                .data_file_path(&part1.id, part1.created_at)
                .unwrap()
                .display()
                .to_string(),
        );
        let mut part1_file = File::create(part1.path.clone().unwrap()).unwrap();
        part1.length = Some(part1_file.write(b"hello ").unwrap());

        parts.push(part1);

        let mut part2 = FileInfo::new("part_id2", None, None, storage.to_string(), None);
        part2.path = Some(
            storage
                .data_file_path(&part2.id, part2.created_at)
                .unwrap()
                .display()
                .to_string(),
        );
        let mut part2_file = File::create(part2.path.clone().unwrap()).unwrap();
        part2.length = Some(part2_file.write(b"world").unwrap());

        parts.push(part2);

        let mut final_info = FileInfo::new("final_id", None, None, storage.to_string(), None);
        final_info.path = Some(
            storage
                .data_file_path(&final_info.id, final_info.created_at)
                .unwrap()
                .display()
                .to_string(),
        );

        storage.concat_files(&final_info, parts).await.unwrap();
        let mut final_file = File::open(final_info.path.unwrap()).unwrap();
        let mut buffer = String::new();
        final_file.read_to_string(&mut buffer).unwrap();

        assert_eq!(buffer.as_str(), "hello world");
    }
}
