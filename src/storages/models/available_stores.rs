use crate::{
    from_str,
    storages::{file_storage, s3_hybrid_storage},
    RustusConf, Storage,
};
use derive_more::{Display, From};
use strum::EnumIter;

/// Enum of available Storage implementations.
#[derive(PartialEq, Eq, From, Display, EnumIter, Clone, Debug)]
pub enum AvailableStores {
    #[display(fmt = "file-storage")]
    FileStorage,
    #[display(fmt = "hybrid-s3")]
    HybridS3,
}

from_str!(AvailableStores, "storage");

impl AvailableStores {
    /// Convert `AvailableStores` to the Storage.
    ///
    /// # Params
    /// `config` - Rustus configuration.
    /// `info_storage` - Storage for information about files.
    ///
    #[cfg_attr(coverage, no_coverage)]
    pub fn get(&self, config: &RustusConf) -> Box<dyn Storage + Send + Sync> {
        #[allow(clippy::single_match)]
        match self {
            Self::FileStorage => Box::new(file_storage::FileStorage::new(
                config.storage_opts.data_dir.clone(),
                config.storage_opts.dir_structure.clone(),
                config.storage_opts.force_fsync,
            )),
            Self::HybridS3 => Box::new(s3_hybrid_storage::S3HybridStorage::new(
                config.storage_opts.s3_url.clone().unwrap(),
                config.storage_opts.s3_region.clone().unwrap(),
                &config.storage_opts.s3_access_key,
                &config.storage_opts.s3_secret_key,
                &config.storage_opts.s3_security_token,
                &config.storage_opts.s3_session_token,
                &config.storage_opts.s3_profile,
                &config.storage_opts.s3_headers,
                config.storage_opts.s3_bucket.clone().unwrap().as_str(),
                config.storage_opts.s3_force_path_style,
                config.storage_opts.data_dir.clone(),
                config.storage_opts.dir_structure.clone(),
                config.storage_opts.force_fsync,
            )),
        }
    }
}
