use crate::info_storages::InfoStorage;
use crate::storages::file_storage;
use crate::{RustusConf, Storage};
use derive_more::{Display, From};
use std::str::FromStr;

/// Enum of available Storage implementations.
#[derive(PartialEq, From, Display, Clone, Debug)]
pub enum AvailableStores {
    #[display(fmt = "FileStorage")]
    FileStorage,
}

impl FromStr for AvailableStores {
    type Err = String;

    /// This function converts string to the `AvailableStore` item.
    /// This function is used by structopt to parse CLI parameters.
    ///
    /// # Params
    /// `input` - input string.
    fn from_str(input: &str) -> Result<AvailableStores, Self::Err> {
        match input {
            "file_storage" => Ok(AvailableStores::FileStorage),
            _ => Err(String::from("Unknown storage type")),
        }
    }
}

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
