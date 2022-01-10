use crate::{InfoStorage, NotificationManager, RustusConf, Storage};

pub struct State {
    pub config: RustusConf,
    pub data_storage: Box<dyn Storage + Send + Sync>,
    pub info_storage: Box<dyn InfoStorage + Send + Sync>,
    pub notification_manager: NotificationManager,
}

impl State {
    pub fn new(
        config: RustusConf,
        data_storage: Box<dyn Storage + Send + Sync>,
        info_storage: Box<dyn InfoStorage + Send + Sync>,
        notification_manager: NotificationManager,
    ) -> Self {
        Self {
            config,
            data_storage,
            info_storage,
            notification_manager,
        }
    }
}
