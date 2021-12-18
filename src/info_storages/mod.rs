use std::str::FromStr;

use async_trait::async_trait;
use derive_more::{Display, From};

pub use file_info::FileInfo;

use crate::errors::RustusResult;
use crate::RustusConf;

mod file_info;

pub mod file_info_storage;


#[derive(PartialEq, From, Display, Clone, Debug)]
pub enum AvailableInfoStores {
    #[display(fmt = "FileStorage")]
    FileInfoStorage,
}

impl FromStr for AvailableInfoStores {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "file_info_storage" => Ok(AvailableInfoStores::FileInfoStorage),
            _ => Err(String::from("Unknown storage type")),
        }
    }
}

impl AvailableInfoStores {
    /// Convert `AvailableInfoStores` to the impl `InfoStorage`.
    ///
    /// # Params
    /// `config` - Rustus configuration.
    ///
    pub fn get(&self, config: &RustusConf) -> Box<dyn InfoStorage + Sync + Send> {
        #[allow(clippy::single_match)]
        match self {
            Self::FileInfoStorage => Box::new(file_info_storage::FileInfoStorage::new(config.clone())),
        }
    }
}

#[async_trait]
pub trait InfoStorage {
    async fn prepare(&mut self) -> RustusResult<()>;
    async fn set_info(&self, file_info: &FileInfo) -> RustusResult<()>;
    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo>;
    async fn remove_info(&self, file_id: &str) -> RustusResult<()>;
}
