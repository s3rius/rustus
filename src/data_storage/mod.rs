use std::{fs::File, io::Read, path::PathBuf};

use base::DataStorage;

use crate::{config::RustusConf, file_info::FileInfo, from_str};

pub mod base;
pub mod impls;

/// Enum of available Storage implementations.
#[derive(PartialEq, Eq, strum::Display, strum::EnumIter, Clone, Debug)]
pub enum AvailableDataStorages {
    #[strum(serialize = "file-storage")]
    FileStorage,
    #[strum(serialize = "hybrid-s3")]
    HybridS3,
    #[strum(serialize = "s3")]
    S3,
}

from_str!(AvailableDataStorages, "storage");

#[derive(Clone, Debug)]
pub enum DataStorageImpl {
    File(impls::file_storage::FileDataStorage),
    S3Hybrid(impls::s3_hybrid::S3HybridDataStorage),
    S3(impls::s3_storage::S3DataStorage),
}

impl AvailableDataStorages {
    /// Convert `AvailableStores` to the Storage.
    ///
    /// # Params
    /// `config` - Rustus configuration.
    /// `info_storage` - Storage for information about files.
    ///
    pub fn get(&self, config: &RustusConf) -> DataStorageImpl {
        #[allow(clippy::single_match)]
        match self {
            Self::FileStorage => DataStorageImpl::File(impls::file_storage::FileDataStorage::new(
                config.storage_opts.data_dir.clone(),
                config.storage_opts.dir_structure.clone(),
                config.storage_opts.force_fsync,
            )),
            Self::HybridS3 => {
                log::warn!("Hybrid S3 is an unstable feature. If you ecounter a problem, please raise an issue: https://github.com/s3rius/rustus/issues.");
                let access_key = from_string_or_path(
                    config.storage_opts.s3_access_key.as_ref(),
                    config.storage_opts.s3_access_key_path.as_ref(),
                );
                let secret_key = from_string_or_path(
                    config.storage_opts.s3_secret_key.as_ref(),
                    config.storage_opts.s3_secret_key_path.as_ref(),
                );
                DataStorageImpl::S3Hybrid(impls::s3_hybrid::S3HybridDataStorage::new(
                    config.storage_opts.s3_url.clone().unwrap(),
                    config.storage_opts.s3_region.clone().unwrap(),
                    Some(&access_key),
                    Some(&secret_key),
                    config.storage_opts.s3_security_token.as_ref(),
                    config.storage_opts.s3_session_token.as_ref(),
                    config.storage_opts.s3_profile.as_ref(),
                    config.storage_opts.s3_headers.as_ref(),
                    config.storage_opts.s3_bucket.clone().unwrap().as_str(),
                    config.storage_opts.s3_force_path_style,
                    config.storage_opts.data_dir.clone(),
                    config.storage_opts.dir_structure.clone(),
                    config.storage_opts.force_fsync,
                ))
            }
            Self::S3 => {
                let access_key = from_string_or_path(
                    config.storage_opts.s3_access_key.as_ref(),
                    config.storage_opts.s3_access_key_path.as_ref(),
                );
                let secret_key = from_string_or_path(
                    config.storage_opts.s3_secret_key.as_ref(),
                    config.storage_opts.s3_secret_key_path.as_ref(),
                );
                DataStorageImpl::S3(impls::s3_storage::S3DataStorage::new(
                    config.storage_opts.s3_url.clone().unwrap(),
                    config.storage_opts.s3_region.clone().unwrap(),
                    Some(&access_key),
                    Some(&secret_key),
                    config.storage_opts.s3_security_token.as_ref(),
                    config.storage_opts.s3_session_token.as_ref(),
                    config.storage_opts.s3_profile.as_ref(),
                    config.storage_opts.s3_headers.as_ref(),
                    config.storage_opts.s3_bucket.clone().unwrap().as_str(),
                    config.storage_opts.s3_force_path_style,
                    config.storage_opts.dir_structure.clone(),
                ))
            }
        }
    }
}

// TODO this should probably be a COW
fn from_string_or_path(variable: Option<&String>, path: Option<&PathBuf>) -> String {
    #[allow(clippy::option_if_let_else)]
    if let Some(variable) = variable {
        variable.to_string()
    } else if let Some(path) = path {
        let file =
            File::open(path).unwrap_or_else(|_| panic!("failed to open path {}", path.display()));
        let mut contents = String::new();
        std::io::BufReader::new(file)
            .read_to_string(&mut contents)
            .unwrap_or_else(|_| panic!("failed to read from path {}", path.display()));
        contents
    } else {
        panic!("can't find {variable:?} or path {path:?}")
    }
}

impl DataStorage for DataStorageImpl {
    fn get_name(&self) -> &'static str {
        match self {
            Self::File(file_data_storage) => file_data_storage.get_name(),
            Self::S3Hybrid(s3_hybrid_data_storage) => s3_hybrid_data_storage.get_name(),
            Self::S3(s3_data_storage) => s3_data_storage.get_name(),
        }
    }

    async fn prepare(&mut self) -> crate::errors::RustusResult<()> {
        match self {
            Self::File(file_data_storage) => file_data_storage.prepare().await,
            Self::S3Hybrid(s3_hybrid_data_storage) => s3_hybrid_data_storage.prepare().await,
            Self::S3(s3_data_storage) => s3_data_storage.prepare().await,
        }
    }

    async fn get_contents(
        &self,
        file_info: &FileInfo,
        request: &actix_web::HttpRequest,
    ) -> crate::errors::RustusResult<actix_web::HttpResponse> {
        match self {
            Self::File(file_data_storage) => {
                file_data_storage.get_contents(file_info, request).await
            }
            Self::S3Hybrid(s3_hybrid_data_storage) => {
                s3_hybrid_data_storage
                    .get_contents(file_info, request)
                    .await
            }
            Self::S3(s3_data_storage) => s3_data_storage.get_contents(file_info, request).await,
        }
    }

    async fn add_bytes(
        &self,
        file_info: &mut FileInfo,
        bytes: bytes::Bytes,
    ) -> crate::errors::RustusResult<()> {
        match self {
            Self::File(file_data_storage) => file_data_storage.add_bytes(file_info, bytes).await,
            Self::S3Hybrid(s3_hybrid_data_storage) => {
                s3_hybrid_data_storage.add_bytes(file_info, bytes).await
            }
            Self::S3(s3_data_storage) => s3_data_storage.add_bytes(file_info, bytes).await,
        }
    }

    async fn create_file(&self, file_info: &mut FileInfo) -> crate::errors::RustusResult<String> {
        match self {
            Self::File(file_data_storage) => file_data_storage.create_file(file_info).await,
            Self::S3Hybrid(s3_hybrid_data_storage) => {
                s3_hybrid_data_storage.create_file(file_info).await
            }
            Self::S3(s3_data_storage) => s3_data_storage.create_file(file_info).await,
        }
    }

    async fn concat_files(
        &self,
        file_info: &FileInfo,
        parts_info: Vec<FileInfo>,
    ) -> crate::errors::RustusResult<()> {
        match self {
            Self::File(file_data_storage) => {
                file_data_storage.concat_files(file_info, parts_info).await
            }
            Self::S3Hybrid(s3_hybrid_data_storage) => {
                s3_hybrid_data_storage
                    .concat_files(file_info, parts_info)
                    .await
            }
            Self::S3(s3_data_storage) => s3_data_storage.concat_files(file_info, parts_info).await,
        }
    }

    async fn remove_file(&self, file_info: &FileInfo) -> crate::errors::RustusResult<()> {
        match self {
            Self::File(file_data_storage) => file_data_storage.remove_file(file_info).await,
            Self::S3Hybrid(s3_hybrid_data_storage) => {
                s3_hybrid_data_storage.remove_file(file_info).await
            }
            Self::S3(s3_data_storage) => s3_data_storage.remove_file(file_info).await,
        }
    }
}
