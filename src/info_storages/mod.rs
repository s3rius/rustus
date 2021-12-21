use std::str::FromStr;

use async_trait::async_trait;
use derive_more::{Display, From};

pub use file_info::FileInfo;

use crate::errors::RustusResult;
use crate::RustusConf;

mod file_info;

use strum::{EnumIter, IntoEnumIterator};

#[cfg(feature = "db_info_storage")]
pub mod db_info_storage;
pub mod file_info_storage;
#[cfg(feature = "redis_info_storage")]
pub mod redis_info_storage;

#[derive(PartialEq, From, Display, Clone, Debug, EnumIter)]
pub enum AvailableInfoStores {
    #[display(fmt = "file_info_storage")]
    Files,
    #[cfg(feature = "db_info_storage")]
    #[display(fmt = "db_info_storage")]
    DB,
    #[cfg(feature = "redis_info_storage")]
    #[display(fmt = "redis_info_storage")]
    Redis,
}

impl FromStr for AvailableInfoStores {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let available_stores = AvailableInfoStores::iter()
            .map(|info_store| format!("\t* {}", info_store.to_string()))
            .collect::<Vec<String>>()
            .join("\n");
        let inp_string = String::from(input);
        for store in AvailableInfoStores::iter() {
            if inp_string == store.to_string() {
                return Ok(store);
            }
        }
        Err(format!(
            "Unknown info storage type.\n Available storages:\n{}",
            available_stores
        ))
    }
}

impl AvailableInfoStores {
    /// Convert `AvailableInfoStores` to the impl `InfoStorage`.
    ///
    /// # Params
    /// `config` - Rustus configuration.
    ///
    pub async fn get(
        &self,
        config: &RustusConf,
    ) -> RustusResult<Box<dyn InfoStorage + Sync + Send>> {
        match self {
            Self::Files => Ok(Box::new(file_info_storage::FileInfoStorage::new(
                config.clone(),
            ))),
            #[cfg(feature = "db_info_storage")]
            Self::DB => Ok(Box::new(
                db_info_storage::DBInfoStorage::new(config.clone()).await?,
            )),
            #[cfg(feature = "redis_info_storage")]
            AvailableInfoStores::Redis => Ok(Box::new(
                redis_info_storage::RedisStorage::new(config.clone()).await?,
            )),
        }
    }
}

#[async_trait]
pub trait InfoStorage {
    async fn prepare(&mut self) -> RustusResult<()>;
    async fn set_info(&self, file_info: &FileInfo, create: bool) -> RustusResult<()>;
    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo>;
    async fn remove_info(&self, file_id: &str) -> RustusResult<()>;
}
