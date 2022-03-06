use crate::storages::file_storage;
use crate::{from_str, RustusConf, Storage};
use derive_more::{Display, From};
use strum::EnumIter;

/// Enum of available Storage implementations.
#[derive(PartialEq, From, Display, EnumIter, Clone, Debug)]
pub enum AvailableStores {
    #[display(fmt = "file-storage")]
    FileStorage,
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
        }
    }
}
