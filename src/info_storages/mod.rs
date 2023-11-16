use crate::{config::Config, from_str};

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
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        Ok(Self::Redis(
            redis_info_storage::RedisStorage::new("redis://localhost", None).await?,
        ))
    }
}

impl base::InfoStorage for InfoStorageImpl {
    async fn prepare(&mut self) -> anyhow::Result<()> {
        match self {
            Self::Redis(redis) => redis.prepare().await,
            Self::File(file) => file.prepare().await,
        }
    }

    async fn set_info(
        &self,
        file_info: &crate::models::file_info::FileInfo,
        create: bool,
    ) -> anyhow::Result<()> {
        match self {
            Self::Redis(redis) => redis.set_info(file_info, create).await,
            Self::File(file) => file.set_info(file_info, create).await,
        }
    }

    async fn get_info(&self, file_id: &str) -> anyhow::Result<crate::models::file_info::FileInfo> {
        match self {
            Self::Redis(redis) => redis.get_info(file_id).await,
            Self::File(file) => file.get_info(file_id).await,
        }
    }

    async fn remove_info(&self, file_id: &str) -> anyhow::Result<()> {
        match self {
            Self::Redis(redis) => redis.remove_info(file_id).await,
            Self::File(file) => file.remove_info(file_id).await,
        }
    }
}
