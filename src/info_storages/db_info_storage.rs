use std::time::Duration;

use async_trait::async_trait;
use rbatis::crud::CRUD;
use rbatis::crud_table;
use rbatis::db::DBPoolOptions;
use rbatis::executor::Executor;
use rbatis::rbatis::Rbatis;

use crate::errors::{RustusError, RustusResult};
use crate::info_storages::{FileInfo, InfoStorage};

#[crud_table]
struct DbModel {
    pub id: String,
    pub info: String,
}

impl TryFrom<&FileInfo> for DbModel {
    type Error = RustusError;

    fn try_from(value: &FileInfo) -> Result<Self, Self::Error> {
        Ok(DbModel {
            id: value.id.clone(),
            info: serde_json::to_string(value)?,
        })
    }
}

pub struct DBInfoStorage {
    db: Rbatis,
}

impl DBInfoStorage {
    pub async fn new(dsn: &str) -> RustusResult<Self> {
        let db = Rbatis::new();
        let mut opts = DBPoolOptions::new();
        opts.connect_timeout = Duration::new(2, 0);
        db.link_opt(dsn, opts).await?;
        Ok(Self { db })
    }
}

#[async_trait]
impl InfoStorage for DBInfoStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        self.db
            .exec(
                "CREATE TABLE IF NOT EXISTS db_model (id VARCHAR(40) PRIMARY KEY, info TEXT);",
                Vec::new(),
            )
            .await?;
        Ok(())
    }

    async fn set_info(&self, file_info: &FileInfo, create: bool) -> RustusResult<()> {
        let model = DbModel::try_from(file_info)?;
        if create {
            self.db.save(&model, &[]).await?;
        } else {
            self.db.update_by_column("id", &model).await?;
        }
        Ok(())
    }

    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo> {
        let model: Option<DbModel> = self.db.fetch_by_column("id", file_id).await?;
        if let Some(info) = model {
            FileInfo::from_json(info.info.to_string()).await
        } else {
            Err(RustusError::FileNotFound)
        }
    }

    async fn remove_info(&self, file_id: &str) -> RustusResult<()> {
        self.db
            .remove_by_column::<DbModel, &str>("id", file_id)
            .await?;
        Ok(())
    }
}

#[cfg(feature = "test_db")]
#[cfg(test)]
mod tests {
    use super::{DBInfoStorage, DbModel};
    use crate::info_storages::FileInfo;
    use crate::InfoStorage;
    use rbatis::crud::CRUD;

    async fn get_info_storage() -> DBInfoStorage {
        let db_url = std::env::var("TEST_DB_URL").unwrap();
        let mut storage = DBInfoStorage::new(db_url.as_str()).await.unwrap();
        storage.prepare().await.unwrap();
        storage
    }

    #[actix_rt::test]
    async fn success() {
        let info_storage = get_info_storage().await;
        let file_info = FileInfo::new_test();
        info_storage.set_info(&file_info, true).await.unwrap();
        let info = info_storage
            .db
            .fetch_by_column::<Option<DbModel>, &str>("id", file_info.id.as_str())
            .await
            .unwrap();
        assert!(info.is_some());
        let info = info_storage.get_info(file_info.id.as_str()).await.unwrap();
        assert_eq!(file_info.id, info.id);
        assert_eq!(file_info.storage, info.storage);
        assert_eq!(file_info.length, info.length);
    }

    #[actix_rt::test]
    async fn success_deletion() {
        let info_storage = get_info_storage().await;
        let file_info = FileInfo::new_test();
        info_storage.set_info(&file_info, true).await.unwrap();
        info_storage
            .remove_info(file_info.id.as_str())
            .await
            .unwrap();
        let info = info_storage
            .db
            .fetch_by_column::<Option<DbModel>, &str>("id", file_info.id.as_str())
            .await
            .unwrap();
        assert!(info.is_none());
    }

    #[actix_rt::test]
    async fn deletion_not_found() {
        let info_storage = get_info_storage().await;
        let res = info_storage.remove_info("unknown").await;
        // We don't care if it doesn't exist.
        assert!(res.is_ok());
    }

    #[actix_rt::test]
    async fn getting_not_found() {
        let info_storage = get_info_storage().await;
        let res = info_storage.get_info("unknown").await;
        assert!(res.is_err());
    }
}
