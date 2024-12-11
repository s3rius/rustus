use crate::{errors::RustusResult, RustusConf};
use actix_web::http::header::HeaderMap;
use log::debug;

use super::{
    base::Notifier,
    hooks::Hook,
    impls::{
        amqp_notifier::AMQPNotifier, dir_notifier::DirNotifier, file_notifier::FileNotifier,
        http_notifier::HttpNotifier,
    },
};

#[derive(Clone)]
pub struct NotificationManager {
    notifiers: Vec<NotifierImpl>,
}

#[derive(Clone)]
pub enum NotifierImpl {
    File(FileNotifier),
    Dir(DirNotifier),
    Http(HttpNotifier),
    Amqp(AMQPNotifier),
}

impl NotificationManager {
    pub async fn new(rustus_config: &RustusConf) -> RustusResult<Self> {
        let mut manager = Self {
            notifiers: Vec::new(),
        };
        debug!("Initializing notification manager.");
        if rustus_config.notification_opts.hooks_file.is_some() {
            debug!("Found hooks file");
            manager.notifiers.push(NotifierImpl::File(FileNotifier::new(
                rustus_config.notification_opts.hooks_file.clone().unwrap(),
            )));
        }
        if rustus_config.notification_opts.hooks_dir.is_some() {
            debug!("Found hooks directory");
            manager.notifiers.push(NotifierImpl::Dir(DirNotifier::new(
                rustus_config.notification_opts.hooks_dir.clone().unwrap(),
            )));
        }
        if !rustus_config.notification_opts.hooks_http_urls.is_empty() {
            debug!("Found http hook urls.");
            manager.notifiers.push(NotifierImpl::Http(HttpNotifier::new(
                rustus_config.notification_opts.hooks_http_urls.clone(),
                rustus_config
                    .notification_opts
                    .hooks_http_proxy_headers
                    .clone(),
                rustus_config.notification_opts.http_hook_timeout,
            )));
        }
        if rustus_config.notification_opts.amqp_hook_opts.url.is_some() {
            debug!("Found AMQP notifier.");
            manager.notifiers.push(NotifierImpl::Amqp(AMQPNotifier::new(
                rustus_config.notification_opts.amqp_hook_opts.clone(),
            )));
        }
        for notifier in &mut manager.notifiers.iter_mut() {
            notifier.prepare().await?;
        }
        debug!("Notification manager initialized.");
        Ok(manager)
    }

    pub async fn send_message(
        &self,
        message: String,
        hook: Hook,
        header_map: &HeaderMap,
    ) -> RustusResult<()> {
        log::debug!("Sending a `{}` hook with body `{}`", hook, message);
        for notifier in &self.notifiers {
            notifier
                .send_message(message.clone(), hook, header_map)
                .await?;
        }
        Ok(())
    }
}

impl Notifier for NotifierImpl {
    async fn prepare(&mut self) -> RustusResult<()> {
        match self {
            Self::File(file_notifier) => file_notifier.prepare().await,
            Self::Dir(dir_notifier) => dir_notifier.prepare().await,
            Self::Http(http_notifier) => http_notifier.prepare().await,
            Self::Amqp(amqp_notifier) => amqp_notifier.prepare().await,
        }
    }

    async fn send_message(
        &self,
        message: String,
        hook: Hook,
        headers_map: &HeaderMap,
    ) -> RustusResult<()> {
        match self {
            Self::File(file_notifier) => {
                file_notifier.send_message(message, hook, headers_map).await
            }
            Self::Dir(dir_notifier) => dir_notifier.send_message(message, hook, headers_map).await,
            Self::Http(http_notifier) => {
                http_notifier.send_message(message, hook, headers_map).await
            }
            Self::Amqp(amqp_notifier) => {
                amqp_notifier.send_message(message, hook, headers_map).await
            }
        }
    }
}
