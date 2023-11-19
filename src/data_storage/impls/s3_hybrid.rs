use std::{collections::HashMap, path::PathBuf};

use crate::{
    data_storage::base::Storage,
    errors::{RustusError, RustusResult},
    models::file_info::FileInfo,
    utils::{headers::HeaderMapExt, result::MonadLogger},
};

use crate::utils::dir_struct::substr_time;

use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use s3::{
    command::Command,
    request::{tokio_backend::Reqwest, Request as S3Request},
    Bucket,
};

use super::file_storage::FileStorage;

/// This storage is useful for small files when you have chunks less than 5MB.
/// This restriction is based on the S3 API limitations.
///
/// It handles uploads localy, and after the upload is
/// complete, it uploads file to S3.
///
/// It's not intended to use this storage for large files.
#[derive(Clone)]
pub struct S3HybridStorage {
    bucket: Bucket,
    local_storage: FileStorage,
    dir_struct: String,
}

impl S3HybridStorage {
    /// Create new `S3HybridStorage` instance.
    ///
    /// # Panics
    ///
    /// Might panic if credentials are invalid and cannot be parsed.
    /// Or if bucket instance cannot be created.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        endpoint: String,
        region: String,
        access_key: &Option<String>,
        secret_key: &Option<String>,
        security_token: &Option<String>,
        session_token: &Option<String>,
        profile: &Option<String>,
        custom_headers: &Option<String>,
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
        let credentials = creds.mlog_err("Cannot parse S3 credentials").unwrap();
        let bucket = Bucket::new(
            bucket_name,
            s3::Region::Custom { region, endpoint },
            credentials,
        );
        let mut bucket = bucket.mlog_err("Cannot create bucket instance").unwrap();
        if let Some(raw_s3_headers) = custom_headers {
            let headers_map = serde_json::from_str::<HashMap<String, String>>(raw_s3_headers)
                .mlog_err("Cannot parse s3 headers. Please provide valid JSON object.")
                .unwrap();
            log::debug!("Found extra s3 headers.");
            for (key, value) in &headers_map {
                log::debug!("Adding header `{key}` with value `{value}`.");
                bucket.add_header(key, value);
            }
        }

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
    /// This function is called to upload file to s3 completely.
    /// It streams file directly from disk to s3.
    async fn upload_file(&self, file_info: &FileInfo) -> RustusResult<()> {
        if file_info.path.is_none() {
            return Err(RustusError::UnableToWrite("Cannot get upload path.".into()));
        }
        let s3_path = self.get_s3_key(file_info);
        log::debug!(
            "Starting uploading {} to S3 with key `{}`",
            file_info.id,
            s3_path,
        );
        let file = tokio::fs::File::open(file_info.path.clone().unwrap()).await?;
        let mut reader = tokio::io::BufReader::new(file);
        self.bucket.put_object_stream(&mut reader, s3_path).await?;
        Ok(())
    }

    // Construct an S3 key which is used to upload files.
    fn get_s3_key(&self, file_info: &FileInfo) -> String {
        let base_path = substr_time(self.dir_struct.as_str(), file_info.created_at);
        let trimmed_path = base_path.trim_end_matches(|c: char| c == '/');
        format!("{trimmed_path}/{}", file_info.id)
    }
}

impl Storage for S3HybridStorage {
    fn get_name(&self) -> &'static str {
        "s3_hybrid"
    }

    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    async fn get_contents(&self, file_info: &FileInfo) -> RustusResult<Response> {
        if file_info.length != Some(file_info.offset) {
            log::debug!("File isn't uploaded. Returning from local storage.");
            return self.local_storage.get_contents(file_info).await;
        }
        let key = self.get_s3_key(file_info);
        let command = Command::GetObject;
        let s3_request = Reqwest::new(&self.bucket, &key, command).unwrap();
        let s3_response = s3_request.response().await.unwrap();
        let mut resp = axum::body::Body::from_stream(s3_response.bytes_stream()).into_response();
        resp.headers_mut()
            .generate_disposition(file_info.get_filename());
        Ok(resp)
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
                .delete_object(self.get_s3_key(file_info))
                .await?;
        } else {
            self.local_storage.remove_file(file_info).await?;
        }
        Ok(())
    }
}
