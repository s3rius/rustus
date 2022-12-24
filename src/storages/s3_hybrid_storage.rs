use std::path::PathBuf;

use crate::{
    errors::{RustusError, RustusResult},
    info_storages::FileInfo,
};

use super::Storage;
use crate::{storages::file_storage::FileStorage, utils::dir_struct::dir_struct};
use actix_web::{HttpRequest, HttpResponse, HttpResponseBuilder};
use async_trait::async_trait;
use bytes::Bytes;
use derive_more::Display;
use s3::{command::Command, request::Reqwest, request_trait::Request, Bucket};

/// This storage is useful for small files when you have chunks less than 5MB.
/// This restriction is based on the S3 API limitations.
///
/// It handles uploads localy, and after the upload is
/// complete, it uploads file to S3.
///
/// It's not intended to use this storage for large files.
#[derive(Display, Clone)]
#[display(fmt = "s3_storage")]
pub struct S3Storage {
    bucket: Bucket,
    local_storage: FileStorage,
    dir_struct: String,
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
        let local_storage = FileStorage::new(data_dir, dir_struct.clone(), force_fsync);
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
            dir_struct,
        }
    }

    /// Upload file to S3.
    ///
    /// This function is called with
    async fn upload_file(&self, file_info: &FileInfo) -> RustusResult<()> {
        if file_info.path.is_none() {
            return Err(RustusError::UnableToWrite("Cannot get upload path.".into()));
        }
        let s3_path = self.get_s3_key(file_info.id.as_str());
        let file = tokio::fs::File::open(file_info.path.clone().unwrap()).await?;
        let mut reader = tokio::io::BufReader::new(file);
        self.bucket.put_object_stream(&mut reader, s3_path).await?;
        Ok(())
    }

    // Construct an S3 key which is used to upload files.
    fn get_s3_key(&self, file_id: &str) -> String {
        let base_path = dir_struct(self.dir_struct.as_str());
        let trimmed_path = base_path.trim_end_matches(|c: char| c == '/');
        format!("{trimmed_path}/{file_id}")
    }
}

#[async_trait(?Send)]
impl Storage for S3Storage {
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    async fn get_contents(
        &self,
        file_info: &FileInfo,
        request: &HttpRequest,
    ) -> RustusResult<HttpResponse> {
        if file_info.length != Some(file_info.offset) {
            return self.local_storage.get_contents(file_info, request).await;
        }

        let key = self.get_s3_key(&file_info.id);
        let command = Command::GetObject;
        let s3_request = Reqwest::new(&self.bucket, &key, command);
        let s3_response = s3_request.response().await?;

        let mut response = HttpResponseBuilder::new(actix_web::http::StatusCode::OK);
        Ok(response.streaming(s3_response.bytes_stream()))
    }

    async fn add_bytes(&self, file_info: &FileInfo, bytes: Bytes) -> RustusResult<()> {
        let part_len = bytes.len();
        self.local_storage.add_bytes(file_info, bytes).await?;
        // If upload is complete. Upload the resulting file onto S3.
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
            self.bucket
                .delete_object(self.get_s3_key(&file_info.id))
                .await?;
        } else {
            self.local_storage.remove_file(file_info).await?;
        }
        Ok(())
    }
}
