use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use crate::{config::Config, errors::RustusResult, from_str, utils::result::MonadLogger};

use self::impls::{file_storage::FileStorage, s3_hybrid::S3HybridStorage};

pub mod base;
pub mod impls;

#[derive(Clone, Debug, strum::Display, strum::EnumIter)]
pub enum AvailableStorages {
    #[strum(serialize = "file-storage")]
    File,
    #[strum(serialize = "hybrid-s3")]
    S3Hybrid,
}

from_str!(AvailableStorages, "storages");

#[derive(Clone)]
pub enum DataStorageImpl {
    File(FileStorage),
    S3Hybrid(S3HybridStorage),
}

impl DataStorageImpl {
    /// Create `DataStorage` from config.
    ///
    /// This function creates a generic storage, which might hold any kind of data storage.
    ///
    /// # Panics
    ///
    /// Might panic if one of required fields is not set for `S3Hybrid` storage,
    /// and `S3Hybrid` is selected as data storage.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        let data_conf = config.data_storage_config.clone();
        match data_conf.storage {
            AvailableStorages::File => Self::File(FileStorage::new(
                data_conf.data_dir,
                data_conf.dir_structure,
                data_conf.force_fsync,
            )),
            AvailableStorages::S3Hybrid => {
                let access_key =
                    from_string_or_path(&data_conf.s3_access_key, &data_conf.s3_access_key_path);
                let secret_key =
                    from_string_or_path(&data_conf.s3_secret_key, &data_conf.s3_secret_key_path);
                Self::S3Hybrid(S3HybridStorage::new(
                    data_conf.s3_url.clone().mlog_err("S3 URL").unwrap(),
                    data_conf.s3_region.clone().mlog_err("S3 Region").unwrap(),
                    &Some(access_key),
                    &Some(secret_key),
                    &data_conf.s3_security_token,
                    &data_conf.s3_session_token,
                    &data_conf.s3_profile,
                    &data_conf.s3_headers,
                    data_conf
                        .s3_bucket
                        .clone()
                        .mlog_err("S3 bucket")
                        .unwrap()
                        .as_str(),
                    data_conf.s3_force_path_style,
                    data_conf.data_dir.clone(),
                    data_conf.dir_structure.clone(),
                    data_conf.force_fsync,
                ))
            }
        }
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
    ) -> crate::errors::RustusResult<axum::response::Response> {
        match self {
            Self::File(file) => file.get_contents(file_info).await,
            Self::S3Hybrid(s3) => s3.get_contents(file_info).await,
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
fn from_string_or_path(variable: &Option<String>, path: &Option<PathBuf>) -> String {
    if let Some(variable) = variable {
        variable.to_string()
    } else if let Some(path) = path {
        let file =
            File::open(path).unwrap_or_else(|_| panic!("failed to open path {}", path.display()));
        let mut contents = String::new();
        BufReader::new(file)
            .read_to_string(&mut contents)
            .unwrap_or_else(|_| panic!("failed to read from path {}", path.display()));
        contents
    } else {
        panic!("can't find {variable:?} or path {path:?}")
    }
}
