use crate::{
    config::Config,
    notifiers::impls::{
        amqp_notifier::AMQPNotifier, dir_notifier::DirNotifier, file_notifier::FileNotifier,
        http_notifier::HttpNotifier,
    },
};

use super::{base::Notifier, NotifierImpl};
use axum::http::HeaderMap;

#[derive(Clone, Debug)]
pub struct NotificationManager {
    notifiers: Vec<NotifierImpl>,
}

impl NotificationManager {
    /// Construct a new `NotificationManager`.
    ///
    /// Manager is used to send notification for hooks.
    /// It's capable of having multiple notifiers,
    /// which are used to send messages.
    #[must_use]
    pub fn new(rustus_config: &Config) -> Self {
        let mut manager = Self {
            notifiers: Vec::new(),
        };
        log::debug!("Initializing notification manager.");
        if let Some(hooks_file) = &rustus_config.notification_config.hooks_file {
            log::debug!("Found hooks file");
            manager
                .notifiers
                .push(NotifierImpl::File(FileNotifier::new(hooks_file.clone())));
        }
        if let Some(hooks_dir) = &rustus_config.notification_config.hooks_dir {
            log::debug!("Found hooks directory");
            manager
                .notifiers
                .push(NotifierImpl::Dir(DirNotifier::new(hooks_dir.clone())));
        }
        if !rustus_config.notification_config.hooks_http_urls.is_empty() {
            log::debug!("Found http hook urls.");
            manager.notifiers.push(NotifierImpl::Http(HttpNotifier::new(
                rustus_config.notification_config.hooks_http_urls.clone(),
                rustus_config
                    .notification_config
                    .hooks_http_proxy_headers
                    .clone(),
                rustus_config.notification_config.http_hook_timeout,
            )));
        }
        if rustus_config
            .notification_config
            .amqp_hook_opts
            .hooks_amqp_url
            .is_some()
        {
            log::debug!("Found AMQP notifier.");
            manager.notifiers.push(NotifierImpl::Amqp(AMQPNotifier::new(
                rustus_config.notification_config.amqp_hook_opts.clone(),
            )));
        }
        log::debug!("Notification manager initialized.");
        manager
    }

    /// Prepares all notifiers.
    ///
    /// This function prepares all notifiers for sending messages.
    ///
    /// # Errors
    ///
    /// This method might fail in case if any of the notifiers fails.
    pub async fn prepare(&mut self) -> crate::errors::RustusResult<()> {
        for notifier in &mut self.notifiers {
            notifier.prepare().await?;
        }
        Ok(())
    }

    /// Sends a message to all notifiers.
    ///
    /// # Errors
    ///
    /// This method might fail in case if any of the notifiers fails.
    pub async fn send_message(
        &self,
        message: String,
        hook: super::hooks::Hook,
        headers_map: &HeaderMap,
    ) -> crate::errors::RustusResult<()> {
        for notifier in &self.notifiers {
            notifier
                .send_message(message.clone(), hook, headers_map)
                .await?;
        }
        Ok(())
    }
}
