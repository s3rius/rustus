#[cfg(test)]
use crate::info_storages::FileInfo;
use crate::{data_storage::DataStorageImpl, InfoStorage, NotificationManager, RustusConf};

#[derive(Clone)]
pub struct State {
    pub config: RustusConf,
    pub data_storage: DataStorageImpl,
    pub info_storage: Box<dyn InfoStorage + Send + Sync>,
    pub notification_manager: NotificationManager,
}

impl State {
    pub fn new(
        config: RustusConf,
        info_storage: Box<dyn InfoStorage + Send + Sync>,
        notification_manager: NotificationManager,
    ) -> Self {
        let data_storage = config.storage_opts.storage.get(&config);

        Self {
            config,
            data_storage,
            info_storage,
            notification_manager,
        }
    }

    #[cfg(test)]
    pub async fn from_config_test(config: RustusConf) -> Self {
        Self {
            config: config.clone(),
            data_storage: DataStorageImpl::File(
                crate::data_storage::impls::file_storage::FileDataStorage::new(
                    config.storage_opts.data_dir.clone(),
                    config.storage_opts.dir_structure.clone(),
                    config.storage_opts.force_fsync,
                ),
            ),
            info_storage: Box::new(
                crate::info_storages::file_info_storage::FileInfoStorage::new(
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
        use crate::data_storage::base::DataStorage;

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
