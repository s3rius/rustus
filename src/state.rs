use crate::{
    config::Config,
    data_storage::{base::Storage, DataStorageImpl},
    errors::RustusResult,
    info_storages::{base::InfoStorage, InfoStorageImpl},
    notifiers::NotificationManager,
};

#[derive(Clone)]
pub struct RustusState {
    pub config: Config,
    pub info_storage: InfoStorageImpl,
    pub data_storage: DataStorageImpl,
    pub notificator: NotificationManager,
}

impl RustusState {
    pub async fn from_config(config: &Config) -> RustusResult<Self> {
        let mut info_storage = InfoStorageImpl::new(config).await?;
        let mut data_storage = DataStorageImpl::new(config)?;
        let mut notificator = NotificationManager::new(config)?;
        info_storage.prepare().await?;
        data_storage.prepare().await?;
        notificator.prepare().await?;

        Ok(Self {
            config: config.clone(),
            info_storage,
            data_storage,
            notificator,
        })
    }
}
