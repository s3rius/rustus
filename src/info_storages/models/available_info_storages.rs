use derive_more::{Display, From};

use crate::errors::RustusResult;
use crate::{from_str, RustusConf};

use crate::info_storages::{file_info_storage, InfoStorage};
use strum::EnumIter;

#[cfg(feature = "db_info_storage")]
use crate::info_storages::db_info_storage;

#[cfg(feature = "redis_info_storage")]
use crate::info_storages::redis_info_storage;

#[derive(PartialEq, From, Display, Clone, Debug, EnumIter)]
pub enum AvailableInfoStores {
    #[display(fmt = "file-info-storage")]
    Files,
    #[cfg(feature = "db_info_storage")]
    #[display(fmt = "db-info-storage")]
    DB,
    #[cfg(feature = "redis_info_storage")]
    #[display(fmt = "redis-info-storage")]
    Redis,
}

from_str!(AvailableInfoStores, "info storage");

impl AvailableInfoStores {
    /// Convert `AvailableInfoStores` to the impl `InfoStorage`.
    ///
    /// # Params
    /// `config` - Rustus configuration.
    ///
    #[cfg_attr(coverage, no_coverage)]
    pub async fn get(
        &self,
        config: &RustusConf,
    ) -> RustusResult<Box<dyn InfoStorage + Sync + Send>> {
        match self {
            Self::Files => Ok(Box::new(file_info_storage::FileInfoStorage::new(
                config.info_storage_opts.info_dir.clone(),
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
