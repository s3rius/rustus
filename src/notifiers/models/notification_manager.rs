use crate::errors::RustusResult;
#[cfg(feature = "amqp_notifier")]
use crate::notifiers::amqp_notifier;
#[cfg(feature = "http_notifier")]
use crate::notifiers::http_notifier;
use crate::notifiers::{Hook, Notifier};
use crate::RustusConf;
use actix_web::http::header::HeaderMap;
use log::debug;

pub struct NotificationManager {
    notifiers: Vec<Box<dyn Notifier + Send + Sync>>,
}

impl NotificationManager {
    pub async fn new(tus_config: &RustusConf) -> RustusResult<Self> {
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
                    tus_config
                        .notification_opts
                        .hooks_http_proxy_headers
                        .clone(),
                )));
        }
        #[cfg(feature = "amqp_notifier")]
        if tus_config.notification_opts.hooks_amqp_url.is_some() {
            debug!("Found AMQP notifier.");
            manager
                .notifiers
                .push(Box::new(amqp_notifier::AMQPNotifier::new(
                    tus_config.clone(),
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
        for notifier in &self.notifiers {
            notifier
                .send_message(message.clone(), hook, header_map)
                .await?;
        }
        Ok(())
    }
}
