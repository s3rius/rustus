pub mod base;
pub mod impls;

use derive_more::{Display, From};
use std::str::FromStr;
use strum::IntoEnumIterator;

use crate::{errors::RustusResult, from_str, RustusConf};

use strum::EnumIter;

#[derive(PartialEq, Eq, From, Display, Clone, Debug, EnumIter)]
pub enum AvailableInfoStorages {
    #[display("file-info-storage")]
    Files,
    #[display("redis-info-storage")]
    Redis,
}

from_str!(AvailableInfoStorages, "info storage");

#[derive(Clone, Debug)]
pub enum InfoStorageImpl {
    File(impls::file_storage::FileInfoStorage),
    Redis(impls::redis_storage::RedisInfoStorage),
}

impl AvailableInfoStorages {
    /// Convert `AvailableInfoStores` to the impl `InfoStorage`.
    ///
    /// # Params
    /// `config` - Rustus configuration.
    ///
    pub fn get(&self, config: &RustusConf) -> RustusResult<InfoStorageImpl> {
        match self {
            Self::Files => Ok(InfoStorageImpl::File(
                impls::file_storage::FileInfoStorage::new(
                    config.info_storage_opts.info_dir.clone(),
                ),
            )),
            Self::Redis => Ok(InfoStorageImpl::Redis(
                impls::redis_storage::RedisInfoStorage::new(
                    config
                        .info_storage_opts
                        .info_db_dsn
                        .clone()
                        .unwrap()
                        .as_str(),
                    config.info_storage_opts.redis_info_expiration,
                )?,
            )),
        }
    }
}

impl base::InfoStorage for InfoStorageImpl {
    async fn prepare(&mut self) -> RustusResult<()> {
        match self {
            Self::File(storage) => storage.prepare().await,
            Self::Redis(storage) => storage.prepare().await,
        }
    }

    async fn set_info(
        &self,
        file_info: &crate::file_info::FileInfo,
        create: bool,
    ) -> RustusResult<()> {
        match self {
            Self::File(storage) => storage.set_info(file_info, create).await,
            Self::Redis(storage) => storage.set_info(file_info, create).await,
        }
    }

    async fn get_info(&self, file_id: &str) -> RustusResult<crate::file_info::FileInfo> {
        match self {
            Self::File(storage) => storage.get_info(file_id).await,
            Self::Redis(storage) => storage.get_info(file_id).await,
        }
    }

    async fn remove_info(&self, file_id: &str) -> RustusResult<()> {
        match self {
            Self::File(storage) => storage.remove_info(file_id).await,
            Self::Redis(storage) => storage.remove_info(file_id).await,
        }
    }
}
