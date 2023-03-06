use async_trait::async_trait;
// use mobc_redis::{mobc::Pool, redis, RedisConnectionManager};
// use redis::aio::Connection;

use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::aio::Connection;

use crate::{
    errors::{RustusError, RustusResult},
    info_storages::{FileInfo, InfoStorage},
};

#[derive(Clone)]
pub struct RedisStorage {
    pool: Pool<RedisConnectionManager>,
}

impl RedisStorage {
    #[allow(clippy::unused_async)]
    pub async fn new(db_dsn: &str) -> RustusResult<Self> {
        let manager = RedisConnectionManager::new(db_dsn)?;
        let pool = bb8::Pool::builder().max_size(100).build(manager).await?;
        Ok(Self { pool })
    }
}

#[async_trait(?Send)]
impl InfoStorage for RedisStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    async fn set_info(&self, file_info: &FileInfo, _create: bool) -> RustusResult<()> {
        let mut conn = self.pool.get().await?;
        redis::cmd("SET")
            .arg(file_info.id.as_str())
            .arg(file_info.json().await?.as_str())
            .query_async::<Connection, String>(&mut conn)
            .await
            .map_err(RustusError::from)?;
        Ok(())
    }

    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo> {
        let mut conn = self.pool.get().await?;
        let res = redis::cmd("GET")
            .arg(file_id)
            .query_async::<Connection, Option<String>>(&mut conn)
            .await?;
        if res.is_none() {
            return Err(RustusError::FileNotFound);
        }
        FileInfo::from_json(res.unwrap()).await
    }

    async fn remove_info(&self, file_id: &str) -> RustusResult<()> {
        let mut conn = self.pool.get().await?;
        let resp = redis::cmd("DEL")
            .arg(file_id)
            .query_async::<Connection, Option<usize>>(&mut conn)
            .await?;
        match resp {
            None | Some(0) => Err(RustusError::FileNotFound),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
#[cfg(feature = "test_redis")]
mod tests {
    use super::RedisStorage;
    use crate::{info_storages::FileInfo, InfoStorage};
    use mobc_redis::{redis, redis::AsyncCommands};

    async fn get_storage() -> RedisStorage {
        let redis_url = std::env::var("TEST_REDIS_URL").unwrap();
        RedisStorage::new(redis_url.as_str()).await.unwrap()
    }

    async fn get_redis() -> redis::aio::Connection {
        let redis_url = std::env::var("TEST_REDIS_URL").unwrap();
        let redis = redis::Client::open(redis_url).unwrap();
        redis.get_async_connection().await.unwrap()
    }

    #[actix_rt::test]
    async fn success() {
        let info_storage = get_storage().await;
        let file_info = FileInfo::new_test();
        info_storage.set_info(&file_info, true).await.unwrap();
        let mut redis = get_redis().await;
        let value: Option<String> = redis.get(file_info.id.as_str()).await.unwrap();
        assert!(value.is_some());

        let file_info_from_storage = info_storage.get_info(file_info.id.as_str()).await.unwrap();

        assert_eq!(file_info.id, file_info_from_storage.id);
        assert_eq!(file_info.path, file_info_from_storage.path);
        assert_eq!(file_info.storage, file_info_from_storage.storage);
    }

    #[actix_rt::test]
    async fn no_connection() {
        let info_storage = RedisStorage::new("redis://unknonwn_url/0").await.unwrap();
        let file_info = FileInfo::new_test();
        let res = info_storage.set_info(&file_info, true).await;
        assert!(res.is_err());
    }

    #[actix_rt::test]
    async fn unknown_id() {
        let info_storage = get_storage().await;
        let res = info_storage
            .get_info(uuid::Uuid::new_v4().to_string().as_str())
            .await;
        assert!(res.is_err());
    }

    #[actix_rt::test]
    async fn deletion_success() {
        let info_storage = get_storage().await;
        let mut redis = get_redis().await;
        let res = info_storage.remove_info("unknown").await;
        assert!(res.is_err());
        let file_info = FileInfo::new_test();
        info_storage.set_info(&file_info, true).await.unwrap();
        assert!(redis
            .get::<&str, Option<String>>(file_info.id.as_str())
            .await
            .unwrap()
            .is_some());
        info_storage
            .remove_info(file_info.id.as_str())
            .await
            .unwrap();
        assert!(redis
            .get::<&str, Option<String>>(file_info.id.as_str())
            .await
            .unwrap()
            .is_none());
    }
}
