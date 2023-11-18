use crate::{
    config::Config,
    errors::RustusResult,
    notifiers::impls::{
        amqp_notifier::AMQPNotifier, dir_notifier::DirNotifier, file_notifier::FileNotifier,
        http_notifier::HttpNotifier,
    },
};

use super::{base::Notifier, NotifierImpl};
use axum::http::HeaderMap;

#[derive(Clone)]
pub struct NotificationManager {
    notifiers: Vec<NotifierImpl>,
}

impl NotificationManager {
    pub fn new(rustus_config: &Config) -> RustusResult<Self> {
        let mut manager = Self {
            notifiers: Vec::new(),
        };
        log::debug!("Initializing notification manager.");
        if rustus_config.notification_config.hooks_file.is_some() {
            log::debug!("Found hooks file");
            manager.notifiers.push(NotifierImpl::File(FileNotifier::new(
                rustus_config
                    .notification_config
                    .hooks_file
                    .clone()
                    .unwrap(),
            )));
        }
        if rustus_config.notification_config.hooks_dir.is_some() {
            log::debug!("Found hooks directory");
            manager.notifiers.push(NotifierImpl::Dir(DirNotifier::new(
                rustus_config.notification_config.hooks_dir.clone().unwrap(),
            )));
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
            )?));
        }
        log::debug!("Notification manager initialized.");
        Ok(manager)
    }

    pub async fn prepare(&mut self) -> crate::errors::RustusResult<()> {
        for notifier in &mut self.notifiers {
            notifier.prepare().await?;
        }
        Ok(())
    }

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
