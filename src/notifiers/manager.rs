use crate::{errors::RustusResult, file_info::FileInfo, RustusConf};
use actix_web::http::header::HeaderMap;
use log::debug;

use super::{
    base::Notifier,
    hooks::Hook,
    impls::{
        amqp_notifier::AMQPNotifier, dir_notifier::DirNotifier, file_notifier::FileNotifier,
        http_notifier::HttpNotifier, kafka_notifier::KafkaNotifier, nats_notifier::NatsNotifier,
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
    Kafka(KafkaNotifier),
    Nats(NatsNotifier),
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
        if !rustus_config
            .notification_opts
            .nats_hook_opts
            .urls
            .is_empty()
        {
            debug!("Found NATS notifier.");
            manager.notifiers.push(NotifierImpl::Nats(
                NatsNotifier::new(
                    rustus_config.notification_opts.nats_hook_opts.urls.clone(),
                    rustus_config
                        .notification_opts
                        .nats_hook_opts
                        .subject
                        .clone(),
                    rustus_config
                        .notification_opts
                        .nats_hook_opts
                        .prefix
                        .clone(),
                    rustus_config
                        .notification_opts
                        .nats_hook_opts
                        .wait_for_replies,
                    rustus_config
                        .notification_opts
                        .nats_hook_opts
                        .username
                        .clone(),
                    rustus_config
                        .notification_opts
                        .nats_hook_opts
                        .password
                        .clone(),
                    rustus_config.notification_opts.nats_hook_opts.token.clone(),
                )
                .await?,
            ));
        }
        if rustus_config.notification_opts.amqp_hook_opts.url.is_some() {
            debug!("Found AMQP notifier.");
            manager.notifiers.push(NotifierImpl::Amqp(AMQPNotifier::new(
                rustus_config.notification_opts.amqp_hook_opts.clone(),
            )));
        }
        if let Some(urls) = &rustus_config.notification_opts.kafka_hook_opts.urls {
            let opts = rustus_config.notification_opts.kafka_hook_opts.clone();
            manager
                .notifiers
                .push(NotifierImpl::Kafka(KafkaNotifier::new(
                    urls.to_owned(),
                    opts.client_id,
                    opts.topic,
                    opts.prefix,
                    opts.required_acks,
                    opts.compression,
                    opts.idle_timeout,
                    opts.send_timeout,
                    opts.extra_kafka_opts,
                )?));
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
        file_info: &FileInfo,
        header_map: &HeaderMap,
    ) -> RustusResult<()> {
        log::debug!("Sending a `{}` hook with body `{}`", hook, message);
        for notifier in &self.notifiers {
            notifier
                .send_message(message.clone(), hook, file_info, header_map)
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
            Self::Kafka(kafka_notifier) => kafka_notifier.prepare().await,
            Self::Nats(nats_notifier) => nats_notifier.prepare().await,
        }
    }

    async fn send_message(
        &self,
        message: String,
        hook: Hook,
        file_info: &FileInfo,
        headers_map: &HeaderMap,
    ) -> RustusResult<()> {
        match self {
            Self::File(file_notifier) => {
                file_notifier
                    .send_message(message, hook, file_info, headers_map)
                    .await
            }
            Self::Dir(dir_notifier) => {
                dir_notifier
                    .send_message(message, hook, file_info, headers_map)
                    .await
            }
            Self::Http(http_notifier) => {
                http_notifier
                    .send_message(message, hook, file_info, headers_map)
                    .await
            }
            Self::Amqp(amqp_notifier) => {
                amqp_notifier
                    .send_message(message, hook, file_info, headers_map)
                    .await
            }
            Self::Kafka(kafka_notifier) => {
                kafka_notifier
                    .send_message(message, hook, file_info, headers_map)
                    .await
            }
            Self::Nats(nats_notifier) => {
                nats_notifier
                    .send_message(message, hook, file_info, headers_map)
                    .await
            }
        }
    }
}
