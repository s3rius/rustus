use crate::{config::Config, errors::RustusResult, from_str, utils::result::MonadLogger};

pub mod base;
pub mod impls;

use strum::{Display, EnumIter};

use self::impls::{file_info_storage::FileInfoStorage, redis_info_storage::RedisStorage};

#[derive(Clone, Display, Debug, EnumIter)]
pub enum AvailableInfoStorages {
    #[strum(serialize = "redis-info-storage")]
    Redis,
    #[strum(serialize = "file-info-storage")]
    File,
}

from_str!(AvailableInfoStorages, "info storage");

#[derive(Clone)]
pub enum InfoStorageImpl {
    Redis(RedisStorage),
    File(FileInfoStorage),
}

impl InfoStorageImpl {
    /// Create infoStorage from config.
    ///
    /// This is a generic storage, which might hold any kind of info storage.
    ///
    /// It implements the `InfoStorage` trait as well, so we don't need to use
    /// boxes to call `InfoStorage` methods dynamically.
    ///
    /// # Errors
    ///
    /// Might return an error, if one if info storages cannot be created.
    ///
    /// # Panics
    ///
    /// This function panics if redis dsn is not set, and redis is selected as
    /// info storage.
    pub fn new(config: &Config) -> RustusResult<Self> {
        let info_conf = config.info_storage_config.clone();
        match info_conf.info_storage {
            AvailableInfoStorages::Redis => Ok(Self::Redis(RedisStorage::new(
                info_conf
                    .info_db_dsn
                    .mlog_err("Cannot construct redis. DB DSN")
                    .unwrap()
                    .as_str(),
                info_conf.redis_info_expiration,
            )?)),
            AvailableInfoStorages::File => Ok(Self::File(FileInfoStorage::new(info_conf.info_dir))),
        }
    }
}

impl base::InfoStorage for InfoStorageImpl {
    async fn prepare(&mut self) -> RustusResult<()> {
        match self {
            Self::Redis(redis) => redis.prepare().await,
            Self::File(file) => file.prepare().await,
        }
    }

    async fn set_info(
        &self,
        file_info: &crate::models::file_info::FileInfo,
        create: bool,
    ) -> RustusResult<()> {
        match self {
            Self::Redis(redis) => redis.set_info(file_info, create).await,
            Self::File(file) => file.set_info(file_info, create).await,
        }
    }

    async fn get_info(&self, file_id: &str) -> RustusResult<crate::models::file_info::FileInfo> {
        match self {
            Self::Redis(redis) => redis.get_info(file_id).await,
            Self::File(file) => file.get_info(file_id).await,
        }
    }

    async fn remove_info(&self, file_id: &str) -> RustusResult<()> {
        match self {
            Self::Redis(redis) => redis.remove_info(file_id).await,
            Self::File(file) => file.remove_info(file_id).await,
        }
    }
}
