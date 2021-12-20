use async_trait::async_trait;
use mobc_redis::mobc::Pool;
use mobc_redis::redis;
use mobc_redis::RedisConnectionManager;
use redis::aio::Connection;

use crate::errors::{RustusError, RustusResult};
use crate::info_storages::{FileInfo, InfoStorage};
use crate::RustusConf;

pub struct RedisStorage {
    pool: Pool<RedisConnectionManager>,
}

impl RedisStorage {
    pub async fn new(app_conf: RustusConf) -> RustusResult<Self> {
        let client = redis::Client::open(app_conf.info_storage_opts.info_db_dsn.unwrap().as_str())?;
        let manager = RedisConnectionManager::new(client);
        let pool = Pool::builder().max_open(100).build(manager);
        Ok(Self { pool })
    }
}

#[async_trait]
impl InfoStorage for RedisStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    async fn set_info(&self, file_info: &FileInfo, _create: bool) -> RustusResult<()> {
        let mut conn = self.pool.get().await?;
        redis::cmd("SET")
            .arg(file_info.id.as_str())
            .arg(serde_json::to_string(file_info)?.as_str())
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
        serde_json::from_str(res.unwrap().as_str()).map_err(RustusError::from)
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
