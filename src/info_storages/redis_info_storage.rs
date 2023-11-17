use mobc::{Manager, Pool};
use redis::aio::Connection;

use crate::{
    errors::{RustusError, RustusResult},
    models::file_info::FileInfo,
};

use super::base::InfoStorage;

struct RedisConnectionManager {
    client: redis::Client,
}

impl RedisConnectionManager {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl Manager for RedisConnectionManager {
    type Connection = redis::aio::Connection;
    type Error = redis::RedisError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        Ok(self.client.get_async_connection().await?)
    }

    async fn check(&self, mut conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
        if pong.as_str() != "PONG" {
            return Err((redis::ErrorKind::ResponseError, "pong response error").into());
        }
        Ok(conn)
    }
}

#[derive(Clone)]
pub struct RedisStorage {
    pool: Pool<RedisConnectionManager>,
    expiration: Option<usize>,
}

impl RedisStorage {
    #[allow(clippy::unused_async)]
    pub async fn new(db_dsn: &str, expiration: Option<usize>) -> RustusResult<Self> {
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
            .arg(file_info.json()?.as_str());
        if let Some(expiration) = self.expiration.as_ref() {
            cmd = cmd.arg("EX").arg(expiration);
        }
        cmd.query_async::<Connection, String>(&mut conn).await?;
        Ok(())
    }

    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo> {
        let mut conn = self.pool.get().await?;
        let res = redis::cmd("GET")
            .arg(file_id)
            .query_async::<Connection, Option<String>>(&mut conn)
            .await?;
        if res.is_none() {
            return Err(RustusError::FileNotFound.into());
        }
        FileInfo::from_json(res.unwrap())
    }

    async fn remove_info(&self, file_id: &str) -> RustusResult<()> {
        let mut conn = self.pool.get().await?;
        let resp = redis::cmd("DEL")
            .arg(file_id)
            .query_async::<Connection, Option<usize>>(&mut conn)
            .await?;
        match resp {
            None | Some(0) => Err(RustusError::FileNotFound.into()),
            _ => Ok(()),
        }
    }
}
