use async_trait::async_trait;
use sea_orm::ActiveModelTrait;
use sea_orm::EntityTrait;
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Schema, Set};

use crate::errors::{RustusError, RustusResult};
use crate::info_storages::db_model;
use crate::info_storages::{FileInfo, InfoStorage};
use crate::RustusConf;

pub struct DBInfoStorage {
    db: DatabaseConnection,
}

impl DBInfoStorage {
    pub async fn new(app_conf: RustusConf) -> RustusResult<Self> {
        let db = Database::connect(ConnectOptions::new(
            app_conf.info_storage_opts.info_db_dsn.unwrap().clone(),
        ))
        .await
        .map_err(RustusError::from)?;
        Ok(Self { db })
    }
}

#[async_trait]
impl InfoStorage for DBInfoStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        let builder = self.db.get_database_backend();
        let schema = Schema::new(builder);
        let create_statement = builder.build(
            schema
                .create_table_from_entity(db_model::Entity)
                .if_not_exists(),
        );
        self.db
            .execute(create_statement)
            .await
            .map_err(RustusError::from)?;
        Ok(())
    }

    async fn set_info(&self, file_info: &FileInfo) -> RustusResult<()> {
        let db_model: Option<db_model::Model> = db_model::Entity::find_by_id(file_info.id.clone())
            .one(&self.db)
            .await
            .map_err(RustusError::from)?;

        let model = db_model::Model::try_from(file_info.clone())?;

        if let Some(db_model) = db_model {
            let mut active_model: db_model::ActiveModel = db_model.into();
            active_model.file_info = Set(model.file_info.clone());
            active_model.update(&self.db).await?;
        } else {
            db_model::ActiveModel {
                id: Set(model.id.clone()),
                file_info: Set(model.file_info.clone()),
            }
            .insert(&self.db)
            .await?;
        }

        Ok(())
    }

    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo> {
        let model_opt: Option<db_model::Model> =
            db_model::Entity::find_by_id(String::from(file_id))
                .one(&self.db)
                .await
                .map_err(RustusError::from)?;
        if let Some(model) = model_opt {
            serde_json::from_str(model.file_info.as_str()).map_err(RustusError::from)
        } else {
            Err(RustusError::FileNotFound)
        }
    }

    async fn remove_info(&self, file_id: &str) -> RustusResult<()> {
        let model_opt: Option<db_model::Model> =
            db_model::Entity::find_by_id(String::from(file_id))
                .one(&self.db)
                .await
                .map_err(RustusError::from)?;
        if let Some(model) = model_opt {
            let active_model: db_model::ActiveModel = model.into();
            active_model
                .delete(&self.db)
                .await
                .map_err(RustusError::from)?;
            Ok(())
        } else {
            Err(RustusError::FileNotFound)
        }
    }
}
