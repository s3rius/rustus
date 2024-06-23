use mobc::{Manager, Pool};
use redis::aio::MultiplexedConnection;

use crate::{
    errors::{RustusError, RustusResult},
    info_storages::base::InfoStorage,
    models::file_info::FileInfo,
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
pub struct RedisStorage {
    pool: Pool<RedisConnectionManager>,
    expiration: Option<usize>,
}

impl RedisStorage {
    /// Create new `RedisStorage`.
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

impl InfoStorage for RedisStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    async fn set_info(&self, file_info: &FileInfo, _create: bool) -> RustusResult<()> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("SET");
        let mut cmd = cmd
            .arg(file_info.id.as_str())
            .arg(serde_json::to_string(file_info)?);
        if let Some(expiration) = self.expiration.as_ref() {
            cmd = cmd.arg("EX").arg(expiration);
        }
        cmd.query_async::<MultiplexedConnection, String>(&mut conn)
            .await?;
        Ok(())
    }

    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo> {
        let mut conn = self.pool.get().await?;
        let res = redis::cmd("GET")
            .arg(file_id)
            .query_async::<MultiplexedConnection, Option<String>>(&mut conn)
            .await?;

        res.map_or(Err(RustusError::FileNotFound), |res| {
            serde_json::from_str::<FileInfo>(res.as_str()).map_err(RustusError::from)
        })
    }

    async fn remove_info(&self, file_id: &str) -> RustusResult<()> {
        let mut conn = self.pool.get().await?;
        let resp = redis::cmd("DEL")
            .arg(file_id)
            .query_async::<MultiplexedConnection, Option<usize>>(&mut conn)
            .await?;
        match resp {
            None | Some(0) => Err(RustusError::FileNotFound),
            _ => Ok(()),
        }
    }
}
