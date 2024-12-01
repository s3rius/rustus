#[cfg(test)]
use crate::file_info::FileInfo;
use crate::{
    data_storage::DataStorageImpl, errors::RustusResult, info_storage::InfoStorageImpl,
    NotificationManager, RustusConf,
};

#[derive(Clone)]
pub struct State {
    pub config: RustusConf,
    pub data_storage: DataStorageImpl,
    pub info_storage: InfoStorageImpl,
    pub notification_manager: NotificationManager,
}

impl State {
    pub fn new(
        config: RustusConf,
        notification_manager: NotificationManager,
    ) -> RustusResult<Self> {
        let data_storage = config.storage_opts.storage.get(&config);
        let info_storage = config.info_storage_opts.info_storage.get(&config)?;

        Ok(Self {
            config,
            data_storage,
            info_storage,
            notification_manager,
        })
    }

    #[cfg(test)]
    pub async fn from_config_test(config: RustusConf) -> Self {
        use crate::info_storage::InfoStorageImpl;

        Self {
            config: config.clone(),
            data_storage: DataStorageImpl::File(
                crate::data_storage::impls::file_storage::FileDataStorage::new(
                    config.storage_opts.data_dir.clone(),
                    config.storage_opts.dir_structure.clone(),
                    config.storage_opts.force_fsync,
                ),
            ),
            info_storage: InfoStorageImpl::File(
                crate::info_storage::impls::file_storage::FileInfoStorage::new(
                    config.info_storage_opts.info_dir.clone(),
                ),
            ),
            notification_manager: NotificationManager::new(&config).await.unwrap(),
        }
    }

    #[cfg(test)]
    pub async fn test_new() -> Self {
        let data_dir = tempdir::TempDir::new("data_dir").unwrap();
        let info_dir = tempdir::TempDir::new("info_dir").unwrap();
        let config = RustusConf::from_iter(
            vec![
                "rustus",
                "--data-dir",
                data_dir.into_path().to_str().unwrap(),
                "--info-dir",
                info_dir.into_path().to_str().unwrap(),
            ]
            .into_iter(),
        );
        Self::from_config_test(config).await
    }

    #[cfg(test)]
    pub async fn create_test_file(&self) -> FileInfo {
        use crate::{data_storage::base::DataStorage, info_storage::base::InfoStorage};

        let mut new_file = FileInfo::new(
            uuid::Uuid::new_v4().to_string().as_str(),
            Some(10),
            None,
            self.data_storage.get_name().to_string(),
            None,
        );
        new_file.path = Some(self.data_storage.create_file(&new_file).await.unwrap());
        self.info_storage.set_info(&new_file, true).await.unwrap();
        new_file
    }
}
