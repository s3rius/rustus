use std::collections::HashMap;

use actix_files::NamedFile;
use async_std::fs::{DirBuilder, File};
use async_trait::async_trait;
use log::error;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use thiserror::private::PathAsDisplay;

use crate::errors::{TuserError, TuserResult};
use crate::storages::{FileInfo, Storage};
use crate::TuserConf;

#[derive(Clone)]
pub struct SQLiteFileStorage {
    app_conf: TuserConf,
    pool: Option<SqlitePool>,
}

impl SQLiteFileStorage {
    pub fn new(app_conf: TuserConf) -> SQLiteFileStorage {
        SQLiteFileStorage {
            app_conf,
            pool: None,
        }
    }

    #[allow(dead_code)]
    pub fn get_pool(&self) -> TuserResult<&SqlitePool> {
        if let Some(pool) = &self.pool {
            Ok(pool)
        } else {
            error!("Pool doesn't exist.");
            Err(TuserError::Unknown)
        }
    }
}

#[async_trait]
impl Storage for SQLiteFileStorage {
    async fn prepare(&mut self) -> TuserResult<()> {
        if !self.app_conf.storage_opts.data.exists() {
            DirBuilder::new()
                .create(self.app_conf.storage_opts.data.as_path())
                .await
                .map_err(|err| TuserError::UnableToPrepareStorage(err.to_string()))?;
        }
        if !self.app_conf.storage_opts.sqlite_dsn.exists() {
            File::create(self.app_conf.storage_opts.sqlite_dsn.clone())
                .await
                .map_err(|err| TuserError::UnableToPrepareStorage(err.to_string()))?;
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
            .map_err(TuserError::from)?;
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

    async fn get_file_info(&self, _file_id: &str) -> TuserResult<FileInfo> {
        todo!()
    }

    async fn set_file_info(&self, _file_info: &FileInfo) -> TuserResult<()> {
        todo!()
    }

    async fn get_contents(&self, _file_id: &str) -> TuserResult<NamedFile> {
        todo!()
    }

    async fn add_bytes(
        &self,
        _file_id: &str,
        _request_offset: usize,
        _bytes: &[u8],
    ) -> TuserResult<usize> {
        todo!()
    }

    async fn create_file(
        &self,
        _file_size: Option<usize>,
        _metadata: Option<HashMap<String, String>>,
    ) -> TuserResult<String> {
        todo!()
    }

    async fn remove_file(&self, _file_id: &str) -> TuserResult<()> {
        todo!()
    }
}
