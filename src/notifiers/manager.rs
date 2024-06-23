use crate::{
    config::Config,
    notifiers::impls::{
        amqp_notifier::AMQPNotifier, dir_notifier::DirNotifier, file_notifier::FileNotifier,
        http_notifier::HttpNotifier,
    },
};

use super::{base::Notifier, NotifierImpl};
use axum::http::HeaderMap;
use tracing::Instrument;

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
    #[allow(clippy::cognitive_complexity)]
    pub fn new(rustus_config: &Config) -> Self {
        let mut manager = Self {
            notifiers: Vec::new(),
        };
        tracing::debug!("Initializing notification manager.");
        if let Some(hooks_file) = &rustus_config.notification_config.hooks_file {
            tracing::debug!("Found hooks file");
            manager
                .notifiers
                .push(NotifierImpl::File(FileNotifier::new(hooks_file.clone())));
        }
        if let Some(hooks_dir) = &rustus_config.notification_config.hooks_dir {
            tracing::debug!("Found hooks directory");
            manager
                .notifiers
                .push(NotifierImpl::Dir(DirNotifier::new(hooks_dir.clone())));
        }
        if !rustus_config.notification_config.hooks_http_urls.is_empty() {
            tracing::debug!("Found http hook urls.");
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
            tracing::debug!("Found AMQP notifier.");
            manager.notifiers.push(NotifierImpl::Amqp(AMQPNotifier::new(
                rustus_config.notification_config.amqp_hook_opts.clone(),
            )));
        }
        tracing::debug!("Notification manager initialized.");
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
        tracing::info!("Preparing notifiers.");
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
    #[tracing::instrument(skip(self, message, hook, headers_map))]
    pub async fn notify_all(
        &self,
        message: &str,
        hook: &super::hooks::Hook,
        headers_map: &HeaderMap,
    ) -> crate::errors::RustusResult<()> {
        tracing::info!("Sending message to all notifiers.");
        let collect = self.notifiers.iter().map(|notifier| {
            notifier
                .send_message(message, hook, headers_map)
                .in_current_span()
        });
        futures::future::try_join_all(collect).await?;
        Ok(())
    }
}
