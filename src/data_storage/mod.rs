use crate::{config::Config, errors::RustusResult};

pub mod base;
pub mod file_storage;
pub mod s3_hybrid;

#[derive(Clone)]
pub enum DataStorageImpl {
    File(file_storage::FileStorage),
    S3Hybrid(s3_hybrid::S3HybridStorage),
}

impl DataStorageImpl {
    pub fn new(_config: &Config) -> RustusResult<Self> {
        Ok(Self::File(file_storage::FileStorage::new(
            "./data".into(),
            "{year}/{month}/{day}/".into(),
            false,
        )))
    }
}

impl base::Storage for DataStorageImpl {
    fn get_name(&self) -> &'static str {
        match self {
            Self::File(file) => file.get_name(),
            Self::S3Hybrid(s3) => s3.get_name(),
        }
    }

    async fn prepare(&mut self) -> RustusResult<()> {
        match self {
            Self::File(file) => file.prepare().await,
            Self::S3Hybrid(s3) => s3.prepare().await,
        }
    }

    async fn get_contents(
        &self,
        file_info: &crate::models::file_info::FileInfo,
        request: &axum::extract::Request,
    ) -> crate::errors::RustusResult<axum::response::Response> {
        match self {
            Self::File(file) => file.get_contents(file_info, request).await,
            Self::S3Hybrid(s3) => s3.get_contents(file_info, request).await,
        }
    }

    async fn add_bytes(
        &self,
        file_info: &crate::models::file_info::FileInfo,
        bytes: bytes::Bytes,
    ) -> RustusResult<()> {
        match self {
            Self::File(file) => file.add_bytes(file_info, bytes).await,
            Self::S3Hybrid(s3) => s3.add_bytes(file_info, bytes).await,
        }
    }

    async fn create_file(
        &self,
        file_info: &crate::models::file_info::FileInfo,
    ) -> RustusResult<String> {
        match self {
            Self::File(file) => file.create_file(file_info).await,
            Self::S3Hybrid(s3) => s3.create_file(file_info).await,
        }
    }

    async fn concat_files(
        &self,
        file_info: &crate::models::file_info::FileInfo,
        parts_info: Vec<crate::models::file_info::FileInfo>,
    ) -> RustusResult<()> {
        match self {
            Self::File(file) => file.concat_files(file_info, parts_info).await,
            Self::S3Hybrid(s3) => s3.concat_files(file_info, parts_info).await,
        }
    }

    async fn remove_file(
        &self,
        file_info: &crate::models::file_info::FileInfo,
    ) -> RustusResult<()> {
        match self {
            Self::File(file) => file.remove_file(file_info).await,
            Self::S3Hybrid(s3) => s3.remove_file(file_info).await,
        }
    }
}
