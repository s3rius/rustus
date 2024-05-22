use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use object_store::{
    gcp::{GoogleCloudStorage, GoogleCloudStorageBuilder},
    WriteMultipart,
};
use object_store::{path::Path, ObjectStore};
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

use super::file_storage::FileStorage;
use crate::{
    data_storage::base::Storage,
    errors::{RustusError, RustusResult},
    models::file_info::FileInfo,
    utils::{dir_struct::substr_time, headers::HeaderMapExt, result::MonadLogger},
};

/// It handles uploads localy, and after the upload is
/// complete, it uploads file to GCS.
#[derive(Debug)]
pub struct GCSHybridStorage {
    store: GoogleCloudStorage,
    local_storage: FileStorage,
    dir_struct: String,
}

const UPLOAD_BUFFER_SIZE: usize = 1024 * 1024 * 20; // 20 MB

impl GCSHybridStorage {
    /// Create new `GCSHybridStorage` instance.
    ///
    /// # Panics
    ///
    /// Might panic if credentials are invalid and cannot be parsed.
    /// Or if bucket instance cannot be created.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        service_account_key: Option<String>,
        application_credentials_path: Option<String>,
        bucket_name: &str,
        data_dir: PathBuf,
        dir_struct: String,
        force_fsync: bool,
    ) -> Self {
        let mut store_builder = GoogleCloudStorageBuilder::new().with_bucket_name(bucket_name);

        if let Some(path) = application_credentials_path {
            store_builder = store_builder.with_application_credentials(path);
        }

        if let Some(key) = service_account_key {
            store_builder = store_builder.with_service_account_key(key);
        }

        let store = store_builder
            .build()
            .mlog_err("Cannot create GCS storage")
            .unwrap();

        let local_storage = FileStorage::new(data_dir, dir_struct.clone(), force_fsync);

        Self {
            store,
            local_storage,
            dir_struct,
        }
    }

    /// Upload file to GCS.
    ///
    /// This function is called to upload file to GCS completely.
    /// It streams file directly from disk to GCS.
    async fn upload_file(&self, file_info: &FileInfo) -> RustusResult<()> {
        let file_path = match &file_info.path {
            Some(path) => path.clone(),
            None => return Err(RustusError::UnableToWrite("Cannot get upload path.".into())),
        };

        let key = self.get_gcs_key(file_info);
        tracing::debug!(
            "Starting uploading {} to GCS with key `{}`",
            file_info.id,
            key
        );
        let file = tokio::fs::File::open(file_path).await?;
        let mut reader = tokio::io::BufReader::new(file);

        let upload = self.store.put_multipart(&key).await.map_err(|e| {
            RustusError::UnableToWrite(format!("Failed to start upload of file to GCS: {e}"))
        })?;
        let mut write = WriteMultipart::new(upload);
        let mut buffer = vec![0; UPLOAD_BUFFER_SIZE];

        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            write.write(&buffer[..bytes_read]);
        }

        write
            .finish()
            .await
            .map_err(|_| RustusError::UnableToWrite("Failed to upload file to GCS.".into()))?;

        Ok(())
    }

    // Construct an GCS key which is used to upload files.
    fn get_gcs_key(&self, file_info: &FileInfo) -> Path {
        let base_path = substr_time(self.dir_struct.as_str(), file_info.created_at);
        let trimmed_path = base_path.trim_end_matches(|c: char| c == '/');
        Path::from(format!("{trimmed_path}/{}", file_info.id))
    }
}

impl Storage for GCSHybridStorage {
    fn get_name(&self) -> &'static str {
        "gcs_hybrid"
    }

    async fn prepare(&mut self) -> RustusResult<()> {
        self.local_storage.prepare().await
    }

    async fn get_contents(&self, file_info: &FileInfo) -> RustusResult<Response> {
        if file_info.length != Some(file_info.offset) {
            tracing::debug!("File isn't uploaded. Returning from local storage.");
            return self.local_storage.get_contents(file_info).await;
        }
        let stream = self
            .store
            .get(&self.get_gcs_key(file_info))
            .await
            .unwrap()
            .into_stream();
        let mut resp = axum::body::Body::from_stream(stream).into_response();
        resp.headers_mut()
            .generate_disposition(file_info.get_filename());
        Ok(resp)
    }

    async fn add_bytes(&self, file_info: &FileInfo, bytes: Bytes) -> RustusResult<()> {
        self.local_storage.add_bytes(file_info, bytes).await?;

        if !file_info.is_partial {
            self.upload_file(file_info).await?;
            self.remove_file(file_info).await?;
        }

        Ok(())
    }

    async fn create_file(&self, file_info: &FileInfo) -> RustusResult<String> {
        self.local_storage.create_file(file_info).await
    }

    async fn concat_files(
        &self,
        file_info: &FileInfo,
        parts_info: Vec<FileInfo>,
    ) -> RustusResult<()> {
        self.local_storage
            .concat_files(file_info, parts_info)
            .await?;
        self.upload_file(file_info).await?;
        self.local_storage.remove_file(file_info).await?;
        Ok(())
    }

    async fn remove_file(&self, file_info: &FileInfo) -> RustusResult<()> {
        if Some(file_info.offset) == file_info.length {
            self.store
                .delete(&self.get_gcs_key(file_info))
                .await
                .map_err(|_| {
                    RustusError::UnableToRemove("Failed to delete file from GCS.".into())
                })?;
        } else {
            self.local_storage.remove_file(file_info).await?;
        }
        Ok(())
    }
}
