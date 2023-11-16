use crate::{config::Config, data_storage::DataStorageImpl, info_storages::InfoStorageImpl};

#[derive(Clone)]
pub struct RustusState {
    pub config: Config,
    pub info_storage: InfoStorageImpl,
    pub data_storage: DataStorageImpl,
}

impl RustusState {
    pub async fn from_config(config: &Config) -> anyhow::Result<Self> {
        let info_storage = InfoStorageImpl::new(config).await?;
        let data_storage = DataStorageImpl::new(config)?;
        Ok(Self {
            config: config.clone(),
            info_storage,
            data_storage,
        })
    }
}
