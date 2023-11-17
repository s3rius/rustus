use crate::{config::Config, errors::RustusResult, from_str};

pub mod base;
pub mod file_info_storage;
pub mod redis_info_storage;

use strum::{Display, EnumIter};

#[derive(Clone, Display, Debug, EnumIter)]
pub enum AvailableInfoStorages {
    #[strum(serialize = "redis")]
    Redis,
    #[strum(serialize = "file")]
    File,
}

from_str!(AvailableInfoStorages, "info storage");

#[derive(Clone)]
pub enum InfoStorageImpl {
    Redis(redis_info_storage::RedisStorage),
    File(file_info_storage::FileInfoStorage),
}

impl InfoStorageImpl {
    pub async fn new(_config: &Config) -> RustusResult<Self> {
        Ok(Self::File(file_info_storage::FileInfoStorage::new(
            "./data".into(),
        )))
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
