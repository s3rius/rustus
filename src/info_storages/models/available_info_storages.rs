use derive_more::{Display, From};

use crate::{errors::RustusResult, from_str, RustusConf};

use crate::info_storages::{file_info_storage, InfoStorage};
use strum::EnumIter;

use crate::info_storages::redis_info_storage;

#[derive(PartialEq, Eq, From, Display, Clone, Debug, EnumIter)]
pub enum AvailableInfoStores {
    #[display("file-info-storage")]
    Files,
    #[display("redis-info-storage")]
    Redis,
}

from_str!(AvailableInfoStores, "info storage");

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
                config.info_storage_opts.info_dir.clone(),
            ))),
            AvailableInfoStores::Redis => Ok(Box::new(redis_info_storage::RedisStorage::new(
                config
                    .info_storage_opts
                    .info_db_dsn
                    .clone()
                    .unwrap()
                    .as_str(),
                config.info_storage_opts.redis_info_expiration,
            )?)),
        }
    }
}
