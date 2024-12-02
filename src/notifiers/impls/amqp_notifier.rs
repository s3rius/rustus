use std::time::Duration;

use actix_web::http::header::HeaderMap;
use lapin::{
    options::{BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable, LongString},
    BasicProperties, ConnectionProperties, ExchangeKind,
};
use mobc::Pool;
use strum::IntoEnumIterator;

use crate::{
    config::AMQPHooksOptions,
    errors::RustusResult,
    notifiers::{base::Notifier, hooks::Hook},
    utils::lapin_pool::{ChannelPool, ConnnectionPool},
};

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug)]
pub struct DeclareOptions {
    pub declare_exchange: bool,
    pub durable_exchange: bool,
    pub declare_queues: bool,
    pub durable_queues: bool,
}

#[derive(Clone, Debug)]
pub struct AMQPNotifier {
    exchange_name: String,
    channel_pool: Pool<ChannelPool>,
    queues_prefix: String,
    exchange_kind: String,
    routing_key: Option<String>,
    declare_options: DeclareOptions,
    celery: bool,
}

/// `ManagerConnection` for `ChannelPool`.
///
/// This manager helps you maintain opened channels.
impl AMQPNotifier {
    /// Create new `AMQPNotifier`.
    ///
    /// This method will create two connection pools for AMQP:
    /// * `connection_pool` - for connections
    /// * `channel_pool` - for channels
    ///
    /// The channels pool uses connection pool to get connections
    /// sometimes.
    ///
    /// # Panics
    ///
    /// This method will panic if `hooks_amqp_url` is not set.
    /// But this should not happen, because it's checked before.
    ///
    /// TODO: add separate type for this structure.
    pub fn new(options: AMQPHooksOptions) -> Self {
        let manager = ConnnectionPool::new(
            options.hooks_amqp_url.unwrap(),
            ConnectionProperties::default(),
        );
        let connection_pool = mobc::Pool::builder()
            .max_idle_lifetime(
                options
                    .hooks_amqp_idle_connection_timeout
                    .map(Duration::from_secs),
            )
            .max_open(options.hooks_amqp_connection_pool_size)
            .build(manager);
        let channel_pool = mobc::Pool::builder()
            .max_idle_lifetime(
                options
                    .hooks_amqp_idle_channels_timeout
                    .map(Duration::from_secs),
            )
            .max_open(options.hooks_amqp_channel_pool_size)
            .build(ChannelPool::new(connection_pool));

        Self {
            channel_pool,
            celery: options.hooks_amqp_celery,
            routing_key: options.hooks_amqp_routing_key,
            declare_options: DeclareOptions {
                declare_exchange: options.hooks_amqp_declare_exchange,
                durable_exchange: options.hooks_amqp_durable_exchange,
                declare_queues: options.hooks_amqp_declare_queues,
                durable_queues: options.hooks_amqp_durable_queues,
            },
            exchange_kind: options.hooks_amqp_exchange_kind,
            exchange_name: options.hooks_amqp_exchange,
            queues_prefix: options.hooks_amqp_queues_prefix,
        }
    }

    /// Generate queue name based on hook type.
    ///
    /// If specific routing key is not empty, it returns it.
    /// Otherwise it will generate queue name based on hook name.
    #[must_use]
    pub fn get_queue_name(&self, hook: &Hook) -> String {
        self.routing_key.as_ref().map_or(
            format!("{}.{hook}", self.queues_prefix.as_str()),
            std::convert::Into::into,
        )
    }
}

impl Notifier for AMQPNotifier {
    async fn prepare(&mut self) -> RustusResult<()> {
        let chan = self.channel_pool.get().await?;
        if self.declare_options.declare_exchange {
            chan.exchange_declare(
                self.exchange_name.as_str(),
                ExchangeKind::Custom(self.exchange_kind.clone()),
                ExchangeDeclareOptions {
                    durable: self.declare_options.durable_exchange,
                    ..ExchangeDeclareOptions::default()
                },
                FieldTable::default(),
            )
            .await?;
        }
        if self.declare_options.declare_queues {
            for hook in Hook::iter() {
                let queue_name = self.get_queue_name(&hook);
                chan.queue_declare(
                    queue_name.as_str(),
                    QueueDeclareOptions {
                        durable: self.declare_options.durable_queues,
                        ..QueueDeclareOptions::default()
                    },
                    FieldTable::default(),
                )
                .await?;
                chan.queue_bind(
                    queue_name.as_str(),
                    self.exchange_name.as_str(),
                    queue_name.as_str(),
                    QueueBindOptions::default(),
                    FieldTable::default(),
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn send_message(
        &self,
        message: String,
        hook: Hook,
        _header_map: &HeaderMap,
    ) -> RustusResult<()> {
        log::info!("Sending message to AMQP.");
        let chan = self.channel_pool.get().await?;
        let queue = self.get_queue_name(&hook);
        let routing_key = self.routing_key.as_ref().unwrap_or(&queue);
        let payload = if self.celery {
            format!("[[{message}], {{}}, {{}}]").as_bytes().to_vec()
        } else {
            message.as_bytes().to_vec()
        };
        let mut headers = FieldTable::default();
        if self.celery {
            headers.insert(
                "id".into(),
                AMQPValue::LongString(LongString::from(uuid::Uuid::new_v4().to_string())),
            );
            headers.insert(
                "task".into(),
                AMQPValue::LongString(LongString::from(format!("rustus.{hook}"))),
            );
        }
        chan.basic_publish(
            self.exchange_name.as_str(),
            routing_key.as_str(),
            BasicPublishOptions::default(),
            payload.as_slice(),
            BasicProperties::default()
                .with_headers(headers)
                .with_content_type("application/json".into())
                .with_content_encoding("utf-8".into()),
        )
        .await?;
        Ok(())
    }
}

#[cfg(feature = "test_rmq")]
#[cfg(test)]
mod tests {
    use crate::notifiers::{base::Notifier, hooks::Hook};

    use super::AMQPNotifier;
    use actix_web::http::header::HeaderMap;
    use lapin::options::{BasicAckOptions, BasicGetOptions};

    async fn get_notifier() -> AMQPNotifier {
        let amqp_url = std::env::var("TEST_AMQP_URL").unwrap();
        let mut notifier = AMQPNotifier::new(crate::config::AMQPHooksOptions {
            hooks_amqp_url: Some(amqp_url),
            hooks_amqp_declare_exchange: true,
            hooks_amqp_declare_queues: true,
            hooks_amqp_durable_exchange: false,
            hooks_amqp_durable_queues: false,
            hooks_amqp_celery: true,
            hooks_amqp_exchange: uuid::Uuid::new_v4().to_string(),
            hooks_amqp_exchange_kind: String::from("topic"),
            hooks_amqp_routing_key: None,
            hooks_amqp_queues_prefix: uuid::Uuid::new_v4().to_string(),
            hooks_amqp_connection_pool_size: 1,
            hooks_amqp_channel_pool_size: 1,
            hooks_amqp_idle_connection_timeout: None,
            hooks_amqp_idle_channels_timeout: None,
        });
        notifier.prepare().await.unwrap();
        notifier
    }

    #[actix_rt::test]
    async fn success() {
        let notifier = get_notifier().await;
        let hook = Hook::PostCreate;
        let test_msg = String::from("Test Message");
        notifier
            .send_message(test_msg.clone(), hook.clone(), &HeaderMap::new())
            .await
            .unwrap();
        let chan = notifier.channel_pool.get().await.unwrap();
        let message = chan
            .basic_get(
                format!("{}.{}", notifier.queues_prefix.as_str(), hook).as_str(),
                BasicGetOptions::default(),
            )
            .await
            .unwrap();
        assert!(message.is_some());
        assert_eq!(
            String::from_utf8(message.as_ref().unwrap().data.clone()).unwrap(),
            format!("[[{}], {{}}, {{}}]", test_msg)
        );
        message
            .unwrap()
            .ack(BasicAckOptions::default())
            .await
            .unwrap();
    }

    #[actix_rt::test]
    async fn unknown_url() {
        let notifier = AMQPNotifier::new(crate::config::AMQPHooksOptions {
            hooks_amqp_url: Some(String::from("http://unknown")),
            hooks_amqp_declare_exchange: true,
            hooks_amqp_declare_queues: true,
            hooks_amqp_durable_exchange: false,
            hooks_amqp_durable_queues: false,
            hooks_amqp_celery: false,
            hooks_amqp_exchange: uuid::Uuid::new_v4().to_string(),
            hooks_amqp_exchange_kind: String::from("topic"),
            hooks_amqp_routing_key: None,
            hooks_amqp_queues_prefix: uuid::Uuid::new_v4().to_string(),
            hooks_amqp_connection_pool_size: 1,
            hooks_amqp_channel_pool_size: 1,
            hooks_amqp_idle_connection_timeout: None,
            hooks_amqp_idle_channels_timeout: None,
        })
        .await
        .unwrap();
        let res = notifier
            .send_message("Test Message".into(), Hook::PostCreate, &HeaderMap::new())
            .await;
        assert!(res.is_err());
    }
}
