use std::collections::HashMap;

use actix_files::NamedFile;
use async_std::fs::{DirBuilder, File};
use async_trait::async_trait;
use log::error;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use thiserror::private::PathAsDisplay;

use crate::errors::{RustusError, RustusResult};
use crate::storages::{FileInfo, Storage};
use crate::RustusConf;

#[derive(Clone)]
pub struct SQLiteFileStorage {
    app_conf: RustusConf,
    pool: Option<SqlitePool>,
}

impl SQLiteFileStorage {
    pub fn new(app_conf: RustusConf) -> SQLiteFileStorage {
        SQLiteFileStorage {
            app_conf,
            pool: None,
        }
    }

    #[allow(dead_code)]
    pub fn get_pool(&self) -> RustusResult<&SqlitePool> {
        if let Some(pool) = &self.pool {
            Ok(pool)
        } else {
            error!("Pool doesn't exist.");
            Err(RustusError::Unknown)
        }
    }
}

#[async_trait]
impl Storage for SQLiteFileStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        if !self.app_conf.storage_opts.data.exists() {
            DirBuilder::new()
                .create(self.app_conf.storage_opts.data.as_path())
                .await
                .map_err(|err| RustusError::UnableToPrepareStorage(err.to_string()))?;
        }
        if !self.app_conf.storage_opts.sqlite_dsn.exists() {
            File::create(self.app_conf.storage_opts.sqlite_dsn.clone())
                .await
                .map_err(|err| RustusError::UnableToPrepareStorage(err.to_string()))?;
        }
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(
                format!(
                    "sqlite://{}",
                    self.app_conf
                        .storage_opts
                        .sqlite_dsn
                        .as_display()
                        .to_string()
                )
                .as_str(),
            )
            .await
            .map_err(RustusError::from)?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS \
                fileinfo(\
                    id VARCHAR(40) PRIMARY KEY, \
                    offset UNSIGNED BIG INT NOT NULL DEFAULT 0, \
                    length UNSIGNED BIG INT, \
                    path TEXT, \
                    created_at DATETIME, \
                    deferred_size BOOLEAN, \
                    metadata TEXT\
               );",
        )
        .execute(&pool)
        .await?;
        self.pool = Some(pool);
        Ok(())
    }

    async fn get_file_info(&self, _file_id: &str) -> RustusResult<FileInfo> {
        todo!()
    }

    async fn set_file_info(&self, _file_info: &FileInfo) -> RustusResult<()> {
        todo!()
    }

    async fn get_contents(&self, _file_id: &str) -> RustusResult<NamedFile> {
        todo!()
    }

    async fn add_bytes(
        &self,
        _file_id: &str,
        _request_offset: usize,
        _bytes: &[u8],
    ) -> RustusResult<usize> {
        todo!()
    }

    async fn create_file(
        &self,
        _file_size: Option<usize>,
        _metadata: Option<HashMap<String, String>>,
    ) -> RustusResult<String> {
        todo!()
    }

    async fn remove_file(&self, _file_id: &str) -> RustusResult<()> {
        todo!()
    }
}
