use crate::info_storages::InfoStorage;
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
    ///
    pub fn get(
        &self,
        config: &RustusConf,
        info_storage: Box<dyn InfoStorage + Sync + Send>,
    ) -> Box<dyn Storage + Send + Sync> {
        #[allow(clippy::single_match)]
        match self {
            Self::FileStorage => {
                Box::new(file_storage::FileStorage::new(config.clone(), info_storage))
            }
        }
    }
}
