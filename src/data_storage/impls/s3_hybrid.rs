use std::{collections::HashMap, io::Write, path::PathBuf};

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
use futures::{StreamExt, TryStreamExt};
use s3::{
    command::Command,
    request::{tokio_backend::HyperRequest, Request as S3Request},
    Bucket,
};
use tokio::io::AsyncWriteExt;

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
    concurrent_concat_downloads: usize,
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
        concurrent_concat_downloads: usize,
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
            concurrent_concat_downloads,
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
        format!("{trimmed_path}/{id}")
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

    async fn add_bytes(&self, file_info: &mut FileInfo, bytes: Bytes) -> RustusResult<()> {
        let part_len = bytes.len();
        self.local_storage.add_bytes(file_info, bytes).await?;
        // If upload is complete. Upload the resulting file onto S3.
        if Some(file_info.offset + part_len) == file_info.length {
            self.upload_file(file_info).await?;
            self.local_storage.remove_file(file_info).await?;
        }
        Ok(())
    }

    async fn create_file(&self, file_info: &mut FileInfo) -> RustusResult<String> {
        self.local_storage.create_file(file_info).await?;
        Ok(self.get_s3_key(&file_info.id, file_info.created_at))
    }

    async fn concat_files(
        &self,
        file_info: &FileInfo,
        parts_info: Vec<FileInfo>,
    ) -> RustusResult<()> {
        let dir = tempdir::TempDir::new(&file_info.id)?;
        let mut download_futures = vec![];

        // At first we need to download all parts.
        for part_info in &parts_info {
            let part_key = self.get_s3_key(&part_info.id, part_info.created_at);
            let part_out = dir.path().join(&part_info.id);
            // Here we create a future which downloads the part
            // into a temporary file.
            download_futures.push(async move {
                let part_file = tokio::fs::File::create(&part_out).await?;
                let mut writer = tokio::io::BufWriter::new(part_file);
                let mut reader = self.bucket.get_object_stream(&part_key).await?;
                while let Some(chunk) = reader.bytes().next().await {
                    let mut chunk = chunk?;
                    writer.write_all_buf(&mut chunk).await.map_err(|err| {
                        log::error!("{:?}", err);
                        RustusError::UnableToWrite(err.to_string())
                    })?;
                }
                writer.flush().await?;
                writer.get_ref().sync_data().await?;
                Ok::<_, RustusError>(())
            });
        }
        // Here we await all download futures.
        // We use buffer_unordered to limit the number of concurrent downloads.
        futures::stream::iter(download_futures)
            // Number of concurrent downloads.
            .buffer_unordered(self.concurrent_concat_downloads)
            // We use try_collect to collect all results
            // and return an error if any of the futures returned an error.
            .try_collect::<Vec<_>>()
            .await?;

        let output_path = dir.path().join(&file_info.id);
        let output_path_cloned = output_path.clone();
        let parts_files = parts_info
            .iter()
            .map(|info| dir.path().join(&info.id))
            .collect::<Vec<_>>();
        tokio::task::spawn_blocking(move || {
            let file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(output_path_cloned)
                .map_err(|err| {
                    log::error!("{:?}", err);
                    RustusError::UnableToWrite(err.to_string())
                })?;
            let mut writer = std::io::BufWriter::new(file);
            for part in &parts_files {
                let part_file = std::fs::OpenOptions::new().read(true).open(part)?;
                let mut reader = std::io::BufReader::new(part_file);
                std::io::copy(&mut reader, &mut writer)?;
            }
            writer.flush()?;
            writer.get_ref().sync_data()?;
            Ok::<_, RustusError>(())
        })
        .await??;

        // We reopen the file to upload it to S3.
        // This is needed because we need to open the file in read mode.
        let output_file = tokio::fs::File::open(&output_path).await?;
        let mut reader = tokio::io::BufReader::new(output_file);
        let key = self.get_s3_key(&file_info.id, file_info.created_at);
        self.bucket.put_object_stream(&mut reader, key).await?;

        tokio::fs::remove_file(output_path).await?;

        Ok(())
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
