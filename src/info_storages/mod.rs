use std::str::FromStr;

use async_trait::async_trait;
use derive_more::{Display, From};

pub use file_info::FileInfo;

use crate::errors::RustusResult;
use crate::RustusConf;

mod file_info;

pub mod db_info_storage;
pub mod file_info_storage;
pub mod redis_info_storage;

#[derive(PartialEq, From, Display, Clone, Debug)]
pub enum AvailableInfoStores {
    #[display(fmt = "File info storage")]
    Files,
    #[display(fmt = "DB info storage")]
    DB,
    #[display(fmt = "Redis info storage")]
    Redis,
}

impl FromStr for AvailableInfoStores {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "file_info_storage" => Ok(AvailableInfoStores::Files),
            "db_info_storage" => Ok(AvailableInfoStores::DB),
            "redis_info_storage" => Ok(AvailableInfoStores::Redis),
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
    pub async fn get(
        &self,
        config: &RustusConf,
    ) -> RustusResult<Box<dyn InfoStorage + Sync + Send>> {
        match self {
            Self::Files => Ok(Box::new(file_info_storage::FileInfoStorage::new(
                config.clone(),
            ))),
            Self::DB => Ok(Box::new(
                db_info_storage::DBInfoStorage::new(config.clone()).await?,
            )),
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
