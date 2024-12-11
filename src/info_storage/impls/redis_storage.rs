use mobc::{Manager, Pool};

use crate::{
    errors::{RustusError, RustusResult},
    file_info::FileInfo,
    info_storage::base::InfoStorage,
};

struct RedisConnectionManager {
    client: redis::Client,
}

impl RedisConnectionManager {
    pub const fn new(client: redis::Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl Manager for RedisConnectionManager {
    type Connection = redis::aio::MultiplexedConnection;
    type Error = redis::RedisError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        Ok(self.client.get_multiplexed_async_connection().await?)
    }

    async fn check(&self, mut conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
        if pong.as_str() != "PONG" {
            return Err((redis::ErrorKind::ResponseError, "pong response error").into());
        }
        Ok(conn)
    }
}

#[derive(Clone, Debug)]
pub struct RedisInfoStorage {
    pool: Pool<RedisConnectionManager>,
    expiration: Option<usize>,
}

impl RedisInfoStorage {
    /// Create new `RedisInfoStorage`.
    ///
    /// # Errors
    ///
    /// Might return an error, if redis client cannot be created.
    pub fn new(db_dsn: &str, expiration: Option<usize>) -> RustusResult<Self> {
        let client = redis::Client::open(db_dsn)?;
        let manager = RedisConnectionManager::new(client);
        let pool = mobc::Pool::builder().max_open(100).build(manager);
        Ok(Self { pool, expiration })
    }
}

impl InfoStorage for RedisInfoStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    async fn set_info(&self, file_info: &FileInfo, _create: bool) -> RustusResult<()> {
        let mut cmd = redis::cmd("SET");
        let mut cmd = cmd
            .arg(file_info.id.as_str())
            .arg(serde_json::to_string(file_info)?);
        if let Some(expiration) = self.expiration.as_ref() {
            cmd = cmd.arg("EX").arg(expiration);
        }
        let mut conn = self.pool.get().await?;
        cmd.query_async::<String>(&mut *conn).await?;
        drop(conn);
        Ok(())
    }

    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo> {
        let mut conn = self.pool.get().await?;
        let res = redis::cmd("GET")
            .arg(file_id)
            .query_async::<Option<String>>(&mut *conn)
            .await?;
        drop(conn);

        res.map_or(Err(RustusError::FileNotFound), |res| {
            serde_json::from_str::<FileInfo>(res.as_str()).map_err(RustusError::from)
        })
    }

    async fn remove_info(&self, file_id: &str) -> RustusResult<()> {
        let mut conn = self.pool.get().await?;
        let resp = redis::cmd("DEL")
            .arg(file_id)
            .query_async::<Option<usize>>(&mut *conn)
            .await?;
        drop(conn);
        match resp {
            None | Some(0) => Err(RustusError::FileNotFound),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{file_info::FileInfo, info_storage::base::InfoStorage};

    use super::RedisInfoStorage;
    use redis::AsyncCommands;

    fn get_url() -> String {
        std::env::var("TEST_REDIS_URL").unwrap_or("redis://localhost/0".to_string())
    }

    async fn get_storage() -> RedisInfoStorage {
        RedisInfoStorage::new(get_url().as_str(), None).unwrap()
    }

    async fn get_redis() -> redis::aio::MultiplexedConnection {
        let redis = redis::Client::open(get_url()).unwrap();
        redis.get_multiplexed_async_connection().await.unwrap()
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
        let info_storage = RedisInfoStorage::new("redis://unknonwn_url/0", None).unwrap();
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
    async fn expiration() {
        let info_storage = get_storage().await;
        let res = info_storage
            .get_info(uuid::Uuid::new_v4().to_string().as_str())
            .await;
        assert!(res.is_err());
    }

    #[actix_rt::test]
    async fn deletion_success() {
        let mut info_storage = get_storage().await;
        info_storage.expiration = Some(1);
        let mut redis = get_redis().await;
        let file_info = FileInfo::new_test();
        info_storage.set_info(&file_info, true).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        assert!(redis
            .get::<&str, Option<String>>(file_info.id.as_str())
            .await
            .unwrap()
            .is_none());
    }
}
