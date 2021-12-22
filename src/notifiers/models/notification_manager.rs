use crate::errors::{RustusError, RustusResult};
#[cfg(feature = "http_notifier")]
use crate::notifiers::http_notifier;
use crate::notifiers::{Hook, Notifier};
use crate::RustusConf;
use futures::future::try_join_all;
use log::debug;

pub struct NotificationManager {
    notifiers: Vec<Box<dyn Notifier + Send + Sync>>,
}

impl NotificationManager {
    pub fn new(tus_config: &RustusConf) -> Self {
        let mut manager = Self {
            notifiers: Vec::new(),
        };
        debug!("Initializing notification manager.");
        #[cfg(feature = "http_notifier")]
        if !tus_config.notification_opts.hooks_http_urls.is_empty() {
            debug!("Found http hook urls.");
            manager
                .notifiers
                .push(Box::new(http_notifier::HttpNotifier::new(
                    tus_config.notification_opts.hooks_http_urls.clone(),
                )));
        }
        debug!("Notification manager initialized.");
        manager
    }

    pub async fn send_message(&self, message: String, hook: Hook) -> RustusResult<()> {
        let mut futures = Vec::new();
        for notifier in &self.notifiers {
            futures.push(notifier.send_message(message.clone(), hook));
        }
        if !futures.is_empty() {
            try_join_all(futures)
                .await
                .map_err(|err| RustusError::HookError(err.to_string()))?;
        }
        Ok(())
    }
}
