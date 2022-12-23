use std::path::PathBuf;

use crate::{
    errors::{RustusError, RustusResult},
    info_storages::FileInfo,
};

use super::Storage;
use crate::storages::file_storage::FileStorage;
use actix_files::NamedFile;
use async_trait::async_trait;
use bytes::Bytes;
use derive_more::Display;
use s3::Bucket;

#[derive(Display, Clone)]
#[display(fmt = "s3_storage")]
pub struct S3Storage {
    bucket: Bucket,
    local_storage: FileStorage,
}

impl S3Storage {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        endpoint: String,
        region: String,
        access_key: &Option<String>,
        secret_key: &Option<String>,
        security_token: &Option<String>,
        session_token: &Option<String>,
        profile: &Option<String>,
        bucket_name: &str,
        force_path_style: bool,
        data_dir: PathBuf,
        dir_struct: String,
        force_fsync: bool,
    ) -> Self {
        let local_storage = FileStorage::new(data_dir, dir_struct, force_fsync);
        let creds = s3::creds::Credentials::new(
            access_key.as_deref(),
            secret_key.as_deref(),
            security_token.as_deref(),
            session_token.as_deref(),
            profile.as_deref(),
        );
        if let Err(err) = creds {
            panic!("Cannot build credentials: {err}")
        }
        let credentials = creds.unwrap();
        let bucket = Bucket::new(
            bucket_name,
            s3::Region::Custom { region, endpoint },
            credentials,
        );
        if let Err(error) = bucket {
            panic!("Cannot create bucket instance {error}");
        }
        let mut bucket = bucket.unwrap();
        if force_path_style {
            bucket = bucket.with_path_style();
        }

        Self {
            bucket,
            local_storage,
        }
    }
}

impl S3Storage {
    async fn upload_file(&self, file_info: &FileInfo) -> RustusResult<()> {
        if file_info.path.is_none() {
            return Err(RustusError::UnableToWrite("Cannot get upload path.".into()));
        }
        let path = file_info.path.clone().unwrap();
        let file = tokio::fs::File::open(path.clone()).await?;
        let mut reader = tokio::io::BufReader::new(file);
        self.bucket.put_object_stream(&mut reader, path).await?;
        Ok(())
    }
}

#[async_trait(?Send)]
impl Storage for S3Storage {
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    async fn get_contents(&self, _file_info: &FileInfo) -> RustusResult<NamedFile> {
        Err(RustusError::Unimplemented(
            "Please read directly from S3.".into(),
        ))
    }

    async fn add_bytes(&self, file_info: &FileInfo, bytes: Bytes) -> RustusResult<()> {
        let part_len = bytes.len();
        self.local_storage.add_bytes(file_info, bytes).await?;
        if Some(file_info.offset + part_len) == file_info.length {
            self.upload_file(file_info).await?;
            self.local_storage.remove_file(file_info).await?;
        }
        Ok(())
    }

    async fn create_file(&self, file_info: &FileInfo) -> RustusResult<String> {
        self.local_storage.create_file(file_info).await
    }

    async fn concat_files(
        &self,
        _file_info: &FileInfo,
        _parts_info: Vec<FileInfo>,
    ) -> RustusResult<()> {
        Err(RustusError::Unimplemented(
            "Hybrid s3 cannot concat files.".into(),
        ))
    }

    async fn remove_file(&self, file_info: &FileInfo) -> RustusResult<()> {
        if Some(file_info.offset) == file_info.length {
            if let Some(path) = &file_info.path {
                self.bucket.delete_object(path.as_str()).await?;
            }
        } else {
            self.local_storage.remove_file(file_info).await?;
        }
        Ok(())
    }
}
