use crate::{config::Config, info_storages::InfoStorageImpl};

#[derive(Clone)]
pub struct RustusState {
    pub config: Config,
    pub info_storage: InfoStorageImpl,
}

impl RustusState {
    pub async fn from_config(config: &Config) -> anyhow::Result<Self> {
        let info_storage = InfoStorageImpl::new(config).await?;
        Ok(Self {
            config: config.clone(),
            info_storage,
        })
    }

}
