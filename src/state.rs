use crate::{
    config::Config,
    data_storage::{base::Storage, DataStorageImpl},
    errors::RustusResult,
    info_storages::{base::InfoStorage, InfoStorageImpl},
};

#[derive(Clone)]
pub struct RustusState {
    pub config: Config,
    pub info_storage: InfoStorageImpl,
    pub data_storage: DataStorageImpl,
}

impl RustusState {
    pub async fn from_config(config: &Config) -> RustusResult<Self> {
        let mut info_storage = InfoStorageImpl::new(config).await?;
        let mut data_storage = DataStorageImpl::new(config)?;
        info_storage.prepare().await?;
        data_storage.prepare().await?;

        Ok(Self {
            config: config.clone(),
            info_storage,
            data_storage,
        })
    }
}
