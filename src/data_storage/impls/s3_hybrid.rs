use std::{collections::HashMap, path::PathBuf};

use crate::{
    data_storage::base::DataStorage,
    errors::{RustusError, RustusResult},
    file_info::FileInfo,
    utils::headers::generate_disposition,
};

use crate::utils::dir_struct::substr_time;

use actix_web::{HttpRequest, HttpResponse, HttpResponseBuilder};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use s3::{
    command::Command,
    request::{tokio_backend::HyperRequest, Request as S3Request},
    Bucket,
};

use super::file_storage::FileDataStorage;

/// This storage is useful for small files when you have chunks less than 5MB.
/// This restriction is based on the S3 API limitations.
///
/// It handles uploads localy, and after the upload is
/// complete, it uploads file to S3.
///
/// It's not intended to use this storage for large files.
#[derive(Clone, Debug)]
pub struct S3HybridDataStorage {
    bucket: Bucket,
    local_storage: FileDataStorage,
    dir_struct: String,
}

impl S3HybridDataStorage {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        endpoint: String,
        region: String,
        access_key: Option<&String>,
        secret_key: Option<&String>,
        security_token: Option<&String>,
        session_token: Option<&String>,
        profile: Option<&String>,
        custom_headers: Option<&String>,
        bucket_name: &str,
        force_path_style: bool,
        data_dir: PathBuf,
        dir_struct: String,
        force_fsync: bool,
    ) -> Self {
        let local_storage = FileDataStorage::new(data_dir, dir_struct.clone(), force_fsync);
        let creds = s3::creds::Credentials::new(
            access_key.map(String::as_str),
            secret_key.map(String::as_str),
            security_token.map(String::as_str),
            session_token.map(String::as_str),
            profile.map(String::as_str),
        );
        if let Err(err) = creds {
            panic!("Cannot build credentials: {err}")
        }
        log::debug!("Parsed credentials");
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
        if let Some(raw_s3_headers) = custom_headers {
            let headers_map = serde_json::from_str::<HashMap<String, String>>(raw_s3_headers)
                .expect("Cannot parse s3 headers. Please provide valid JSON object.");
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
            bucket: *bucket,
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
        let local_path = self
            .local_storage
            .data_file_path(&file_info.id, file_info.created_at)?;
        let s3_path = self.get_s3_key(&file_info.id, file_info.created_at);
        log::debug!(
            "Starting uploading {} to S3 with key `{}`",
            file_info.id,
            s3_path,
        );
        let file = tokio::fs::File::open(local_path).await?;
        let mut reader = tokio::io::BufReader::new(file);
        self.bucket.put_object_stream(&mut reader, s3_path).await?;
        Ok(())
    }

    // Construct an S3 key which is used to upload files.
    fn get_s3_key(&self, id: &str, created_at: DateTime<Utc>) -> String {
        let base_path = substr_time(self.dir_struct.as_str(), created_at);
        let trimmed_path = base_path.trim_end_matches('/');
        format!("{trimmed_path}/{}", id)
    }
}

impl DataStorage for S3HybridDataStorage {
    fn get_name(&self) -> &'static str {
        "s3_storage"
    }
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    async fn get_contents(
        &self,
        file_info: &FileInfo,
        request: &HttpRequest,
    ) -> RustusResult<HttpResponse> {
        if file_info.length != Some(file_info.offset) {
            log::debug!("File isn't uploaded. Returning from local storage.");
            return self.local_storage.get_contents(file_info, request).await;
        }
        let key = self.get_s3_key(&file_info.id, file_info.created_at);
        let command = Command::GetObject;
        let s3_request = HyperRequest::new(&self.bucket, &key, command).await?;
        let s3_response = s3_request.response_data_to_stream().await?;
        let mut response = HttpResponseBuilder::new(actix_web::http::StatusCode::OK);
        Ok(response
            .insert_header(generate_disposition(file_info.get_filename()))
            .streaming(s3_response.bytes))
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
        self.local_storage.create_file(file_info).await?;
        Ok(self.get_s3_key(&file_info.id, file_info.created_at))
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
                .delete_object(self.get_s3_key(&file_info.id, file_info.created_at))
                .await?;
        } else {
            self.local_storage.remove_file(file_info).await?;
        }
        Ok(())
    }
}
