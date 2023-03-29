#[cfg(feature = "amqp_notifier")]
use crate::notifiers::amqp_notifier;
use crate::{
    errors::RustusResult,
    notifiers::{
        dir_notifier::DirNotifier, file_notifier::FileNotifier, http_notifier, Hook, Notifier,
    },
    RustusConf,
};
use actix_web::http::header::HeaderMap;
use log::debug;

#[derive(Clone)]
pub struct NotificationManager {
    notifiers: Vec<Box<dyn Notifier + Send + Sync>>,
}

impl NotificationManager {
    pub async fn new(rustus_config: &RustusConf) -> RustusResult<Self> {
        let mut manager = Self {
            notifiers: Vec::new(),
        };
        debug!("Initializing notification manager.");
        if rustus_config.notification_opts.hooks_file.is_some() {
            debug!("Found hooks file");
            manager.notifiers.push(Box::new(FileNotifier::new(
                rustus_config.notification_opts.hooks_file.clone().unwrap(),
            )));
        }
        if rustus_config.notification_opts.hooks_dir.is_some() {
            debug!("Found hooks directory");
            manager.notifiers.push(Box::new(DirNotifier::new(
                rustus_config.notification_opts.hooks_dir.clone().unwrap(),
            )));
        }
        if !rustus_config.notification_opts.hooks_http_urls.is_empty() {
            debug!("Found http hook urls.");
            manager
                .notifiers
                .push(Box::new(http_notifier::HttpNotifier::new(
                    rustus_config.notification_opts.hooks_http_urls.clone(),
                    rustus_config
                        .notification_opts
                        .hooks_http_proxy_headers
                        .clone(),
                )));
        }
        #[cfg(feature = "amqp_notifier")]
        if rustus_config
            .notification_opts
            .amqp_hook_opts
            .hooks_amqp_url
            .is_some()
        {
            debug!("Found AMQP notifier.");
            manager.notifiers.push(Box::new(
                amqp_notifier::AMQPNotifier::new(
                    rustus_config.notification_opts.amqp_hook_opts.clone(),
                )
                .await?,
            ));
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
        for notifier in &self.notifiers {
            notifier
                .send_message(message.clone(), hook, header_map)
                .await?;
        }
        Ok(())
    }
}
