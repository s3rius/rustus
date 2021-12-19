use sea_orm::entity::prelude::*;

use crate::errors::RustusError;
use crate::info_storages::FileInfo;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "file_info")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub file_info: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl TryFrom<FileInfo> for Model {
    type Error = RustusError;

    fn try_from(value: FileInfo) -> Result<Self, Self::Error> {
        let info_str = serde_json::to_string(&value).map_err(RustusError::from)?;
        Ok(Self {
            id: value.id,
            file_info: info_str,
        })
    }
}
