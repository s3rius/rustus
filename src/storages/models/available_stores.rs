use crate::{
    from_str,
    storages::{file_storage, s3_hybrid_storage},
    RustusConf, Storage,
};
use derive_more::{Display, From};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};
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
            Self::HybridS3 => {
                log::warn!("Hybrid S3 is an unstable feature. If you ecounter a problem, please raise an issue: https://github.com/s3rius/rustus/issues.");
                let access_key = from_string_or_path(
                    &config.storage_opts.s3_access_key,
                    &config.storage_opts.s3_access_key_path,
                );
                let secret_key = from_string_or_path(
                    &config.storage_opts.s3_secret_key,
                    &config.storage_opts.s3_secret_key_path,
                );
                Box::new(s3_hybrid_storage::S3HybridStorage::new(
                    config.storage_opts.s3_url.clone().unwrap(),
                    config.storage_opts.s3_region.clone().unwrap(),
                    &Some(access_key),
                    &Some(secret_key),
                    &config.storage_opts.s3_security_token,
                    &config.storage_opts.s3_session_token,
                    &config.storage_opts.s3_profile,
                    &config.storage_opts.s3_headers,
                    config.storage_opts.s3_bucket.clone().unwrap().as_str(),
                    config.storage_opts.s3_force_path_style,
                    config.storage_opts.data_dir.clone(),
                    config.storage_opts.dir_structure.clone(),
                    config.storage_opts.force_fsync,
                ))
            }
        }
    }
}

// TODO this should probably be a COW
fn from_string_or_path(variable: &Option<String>, path: &Option<PathBuf>) -> String {
    if let Some(variable) = variable {
        variable.to_string()
    } else if let Some(path) = path {
        let file = File::open("path_to_your_file")
            .unwrap_or_else(|_| panic!("failed to open path {}", path.display()));
        let mut contents = String::new();
        BufReader::new(file)
            .read_to_string(&mut contents)
            .unwrap_or_else(|_| panic!("failed to read from path {}", path.display()));
        contents
    } else {
        panic!("can't find {variable:?} or path {path:?}")
    }
}
