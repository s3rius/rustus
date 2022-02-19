use std::path::PathBuf;

use actix_files::NamedFile;
use async_std::fs::{remove_file, DirBuilder, File, OpenOptions};
use async_std::io::{copy, SeekFrom};
use async_std::prelude::*;
use async_trait::async_trait;
use log::error;

use crate::errors::{RustusError, RustusResult};
use crate::info_storages::FileInfo;
use crate::storages::Storage;
use crate::utils::dir_struct::dir_struct;
use derive_more::Display;

#[derive(Display)]
#[display(fmt = "file_storage")]
pub struct FileStorage {
    data_dir: PathBuf,
    dir_struct: String,
    preallocate: bool,
}

impl FileStorage {
    pub fn new(data_dir: PathBuf, dir_struct: String, preallocate: bool) -> FileStorage {
        FileStorage {
            data_dir,
            dir_struct,
            preallocate,
        }
    }

    pub async fn data_file_path(&self, file_id: &str) -> RustusResult<PathBuf> {
        let dir = self
            .data_dir
            // We're working wit absolute paths, because tus.io says so.
            .canonicalize()
            .map_err(|err| {
                error!("{}", err);
                RustusError::UnableToWrite(err.to_string())
            })?
            .join(dir_struct(self.dir_struct.as_str()));
        DirBuilder::new()
            .recursive(true)
            .create(dir.as_path())
            .await
            .map_err(|err| {
                error!("{}", err);
                RustusError::UnableToWrite(err.to_string())
            })?;
        Ok(dir.join(file_id))
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        // We're creating directory for new files
        // if it doesn't already exist.
        if !self.data_dir.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(self.data_dir.as_path())
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
            .append(false)
            .create(false)
            .open(info.path.as_ref().unwrap())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::UnableToWrite(err.to_string())
            })?;
        file.seek(SeekFrom::Start(info.offset as u64))
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::UnableToSeek(info.path.clone().unwrap())
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

        if self.preallocate {
            if let Some(file_length) = file_info.length {
                file.set_len(file_length as u64).await.map_err(|err| {
                    error!("{:?}", err);
                    RustusError::UnableToResize(file_path.display().to_string())
                })?;
            }
        }

        // Let's write an empty string to the beginning of the file.
        // Maybe remove it later.
        file.write_all(b"").await.map_err(|err| {
            error!("{:?}", err);
            RustusError::UnableToWrite(file_path.display().to_string())
        })?;
        file.sync_all().await?;

        Ok(file_path.display().to_string())
    }

    async fn concat_files(
        &self,
        file_info: &FileInfo,
        parts_info: Vec<FileInfo>,
    ) -> RustusResult<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(file_info.path.as_ref().unwrap().clone())
            .await
            .map_err(|err| {
                error!("{:?}", err);
                RustusError::UnableToWrite(err.to_string())
            })?;
        for part in parts_info {
            if part.path.is_none() {
                return Err(RustusError::FileNotFound);
            }
            let mut part_file = File::open(part.path.as_ref().unwrap()).await?;
            copy(&mut part_file, &mut file).await?;
        }
        file.sync_data().await?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::FileStorage;
    use crate::info_storages::FileInfo;
    use crate::Storage;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::path::PathBuf;

    #[actix_rt::test]
    async fn preparation() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let target_path = dir.into_path().join("not_exist");
        let mut storage = FileStorage::new(target_path.clone(), "".into(), false);
        assert_eq!(target_path.exists(), false);
        storage.prepare().await.unwrap();
        assert_eq!(target_path.exists(), true);
    }

    #[actix_rt::test]
    async fn create_file() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), false);
        let file_info = FileInfo::new("test_id", None, None, storage.to_string(), None);
        let new_path = storage.create_file(&file_info).await.unwrap();
        assert!(PathBuf::from(new_path).exists());
    }

    #[actix_rt::test]
    async fn create_file_prealloc() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), true);
        let prealloc_size = 32;
        let file_info = FileInfo::new(
            "test_id",
            Some(prealloc_size),
            None,
            storage.to_string(),
            None,
        );
        let new_path = storage.create_file(&file_info).await.unwrap();
        assert!(PathBuf::from(new_path).exists());
    }

    #[actix_rt::test]
    async fn create_file_prealloc_correct_size() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), true);
        let prealloc_size = 32;
        let file_info = FileInfo::new(
            "test_id",
            Some(prealloc_size),
            None,
            storage.to_string(),
            None,
        );
        let new_path = storage.create_file(&file_info).await.unwrap();
        let file_size = {
            let file = std::fs::File::open(new_path).unwrap();
            file.metadata().unwrap().len() as usize
        };
        assert_eq!(prealloc_size, file_size);
    }

    #[actix_rt::test]
    async fn create_file_but_it_exists() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let base_path = dir.into_path().clone();
        let storage = FileStorage::new(base_path.clone(), "".into(), false);
        let file_info = FileInfo::new("test_id", Some(5), None, storage.to_string(), None);
        File::create(base_path.join("test_id")).unwrap();
        let result = storage.create_file(&file_info).await;
        assert!(result.is_err());
    }

    #[actix_rt::test]
    async fn adding_bytes() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), false);
        let test_data = "MyTestData";
        let mut file_info = FileInfo::new("test_id", None, None, storage.to_string(), None);
        let new_path = storage.create_file(&file_info).await.unwrap();
        file_info.path = Some(new_path.clone());
        storage
            .add_bytes(&file_info, test_data.as_bytes())
            .await
            .unwrap();
        let mut file = File::open(new_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, String::from(test_data))
    }

    #[actix_rt::test]
    async fn adding_bytes_smaller_prealloc() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), true);
        let test_data = "MyTestData";
        let mut file_info = FileInfo::new("test_id", Some(1), None, storage.to_string(), None);
        let new_path = storage.create_file(&file_info).await.unwrap();
        file_info.path = Some(new_path.clone());
        storage
            .add_bytes(&file_info, test_data.as_bytes())
            .await
            .unwrap();
        let mut file = File::open(new_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, String::from(test_data))
    }

    #[actix_rt::test]
    async fn adding_bytes_enough_prealloc() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), true);
        let test_data = "MyTestData";
        let mut file_info = FileInfo::new(
            "test_id",
            Some(test_data.len()),
            None,
            storage.to_string(),
            None,
        );
        let new_path = storage.create_file(&file_info).await.unwrap();
        file_info.path = Some(new_path.clone());
        storage
            .add_bytes(&file_info, test_data.as_bytes())
            .await
            .unwrap();
        let mut file = File::open(new_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, String::from(test_data))
    }

    #[actix_rt::test]
    async fn adding_bytes_in_two_passes() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), false);
        let test_data = vec!["MyTestData", "AnotherData"];
        let mut file_info = FileInfo::new("test_id", None, None, storage.to_string(), None);
        let new_path = storage.create_file(&file_info).await.unwrap();
        file_info.path = Some(new_path.clone());
        storage
            .add_bytes(&file_info, test_data[0].as_bytes())
            .await
            .unwrap();
        file_info.offset += test_data[0].len();
        storage
            .add_bytes(&file_info, test_data[1].as_bytes())
            .await
            .unwrap();
        let mut file = File::open(new_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, test_data.join(""))
    }

    #[actix_rt::test]
    async fn adding_bytes_in_two_passes_small_prealloc() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), true);
        let test_data = vec!["MyTestData", "AnotherData"];
        let mut file_info = FileInfo::new("test_id", Some(1), None, storage.to_string(), None);
        let new_path = storage.create_file(&file_info).await.unwrap();
        file_info.path = Some(new_path.clone());
        storage
            .add_bytes(&file_info, test_data[0].as_bytes())
            .await
            .unwrap();
        file_info.offset += test_data[0].len();
        storage
            .add_bytes(&file_info, test_data[1].as_bytes())
            .await
            .unwrap();
        let mut file = File::open(new_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, test_data.join(""))
    }

    #[actix_rt::test]
    async fn adding_bytes_in_two_passes_enough_prealloc() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), true);
        let test_data = vec!["MyTestData", "AnotherData"];
        let mut file_info = FileInfo::new(
            "test_id",
            Some(test_data.iter().map(|s| s.len()).sum()),
            None,
            storage.to_string(),
            None,
        );
        let new_path = storage.create_file(&file_info).await.unwrap();
        file_info.path = Some(new_path.clone());
        storage
            .add_bytes(&file_info, test_data[0].as_bytes())
            .await
            .unwrap();
        file_info.offset += test_data[0].len();
        storage
            .add_bytes(&file_info, test_data[1].as_bytes())
            .await
            .unwrap();
        let mut file = File::open(new_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, test_data.join(""))
    }

    #[actix_rt::test]
    async fn adding_bytes_to_unknown_file() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), false);
        let test_data = "MyTestData";
        let file_info = FileInfo::new(
            "test_id",
            None,
            Some(String::from("some_file")),
            storage.to_string(),
            None,
        );
        let result = storage.add_bytes(&file_info, test_data.as_bytes()).await;
        assert!(result.is_err())
    }

    #[actix_rt::test]
    async fn get_contents_of_unknown_file() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), false);
        let file_info = FileInfo::new(
            "test_id",
            Some(5),
            Some(storage.data_dir.join("unknown").display().to_string()),
            storage.to_string(),
            None,
        );
        let file_info = storage.get_contents(&file_info).await;
        assert!(file_info.is_err());
    }

    #[actix_rt::test]
    async fn remove_unknown_file() {
        let dir = tempdir::TempDir::new("file_storage").unwrap();
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), false);
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
        let storage = FileStorage::new(dir.into_path().clone(), "".into(), false);

        let mut parts = Vec::new();
        let part1_path = storage.data_dir.as_path().join("part1");
        let mut part1 = File::create(part1_path.clone()).unwrap();
        let size1 = part1.write("hello ".as_bytes()).unwrap();

        parts.push(FileInfo::new(
            "part_id1",
            Some(size1),
            Some(part1_path.display().to_string()),
            storage.to_string(),
            None,
        ));

        let part2_path = storage.data_dir.as_path().join("part2");
        let mut part2 = File::create(part2_path.clone()).unwrap();
        let size2 = part2.write("world".as_bytes()).unwrap();
        parts.push(FileInfo::new(
            "part_id2",
            Some(size2),
            Some(part2_path.display().to_string()),
            storage.to_string(),
            None,
        ));

        let final_info = FileInfo::new(
            "final_id",
            None,
            Some(storage.data_dir.join("final_info").display().to_string()),
            storage.to_string(),
            None,
        );
        storage.concat_files(&final_info, parts).await.unwrap();
        let mut final_file = File::open(final_info.path.unwrap()).unwrap();
        let mut buffer = String::new();
        final_file.read_to_string(&mut buffer).unwrap();

        assert_eq!(buffer.as_str(), "hello world");
    }
}
