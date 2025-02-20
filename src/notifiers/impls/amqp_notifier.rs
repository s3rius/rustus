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
    file_info::FileInfo,
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
    auto_delete: bool,
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
        let manager = ConnnectionPool::new(options.url.unwrap(), ConnectionProperties::default());
        let connection_pool = mobc::Pool::builder()
            .max_idle_lifetime(options.idle_connection_timeout.map(Duration::from_secs))
            .max_open(options.connection_pool_size)
            .build(manager);
        let channel_pool = mobc::Pool::builder()
            .max_idle_lifetime(options.idle_channels_timeout.map(Duration::from_secs))
            .max_open(options.channel_pool_size)
            .build(ChannelPool::new(connection_pool));

        Self {
            channel_pool,
            celery: options.celery,
            routing_key: options.routing_key,
            declare_options: DeclareOptions {
                declare_exchange: options.declare_exchange,
                durable_exchange: options.durable_exchange,
                declare_queues: options.declare_queues,
                durable_queues: options.durable_queues,
            },
            exchange_kind: options.exchange_kind,
            exchange_name: options.exchange,
            queues_prefix: options.queues_prefix,
            auto_delete: options.auto_delete,
        }
    }

    /// Generate queue name based on hook type.
    ///
    /// If specific routing key is not empty, it returns it.
    /// Otherwise it will generate queue name based on hook name.
    #[must_use]
    pub fn get_queue_name(&self, hook: Hook) -> String {
        self.routing_key.as_ref().map_or_else(
            || format!("{}.{hook}", self.queues_prefix.as_str()),
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
                    auto_delete: self.auto_delete,
                    ..ExchangeDeclareOptions::default()
                },
                FieldTable::default(),
            )
            .await?;
        }
        if self.declare_options.declare_queues {
            for hook in Hook::iter() {
                let queue_name = self.get_queue_name(hook);
                chan.queue_declare(
                    queue_name.as_str(),
                    QueueDeclareOptions {
                        durable: self.declare_options.durable_queues,
                        auto_delete: self.auto_delete,
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
            drop(chan);
        }
        Ok(())
    }

    async fn send_message(
        &self,
        message: String,
        hook: Hook,
        _file_info: &FileInfo,
        _header_map: &HeaderMap,
    ) -> RustusResult<()> {
        log::info!("Sending message to AMQP.");
        let queue = self.get_queue_name(hook);
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
        let chan = self.channel_pool.get().await?;
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
        drop(chan);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        file_info::FileInfo,
        notifiers::{base::Notifier, hooks::Hook},
    };

    use super::AMQPNotifier;
    use actix_web::http::header::HeaderMap;
    use lapin::options::{BasicAckOptions, BasicGetOptions};
    use strum::IntoEnumIterator;

    async fn get_notifier() -> AMQPNotifier {
        let amqp_url = std::env::var("TEST_AMQP_URL")
            .unwrap_or("amqp://guest:guest@localhost:5672".to_string());
        let mut notifier = AMQPNotifier::new(crate::config::AMQPHooksOptions {
            url: Some(amqp_url),
            declare_exchange: true,
            declare_queues: true,
            durable_exchange: false,
            durable_queues: false,
            auto_delete: true,
            celery: true,
            exchange: uuid::Uuid::new_v4().to_string(),
            exchange_kind: String::from("topic"),
            routing_key: None,
            queues_prefix: uuid::Uuid::new_v4().to_string(),
            connection_pool_size: 1,
            channel_pool_size: 1,
            idle_connection_timeout: None,
            idle_channels_timeout: None,
        });
        notifier.prepare().await.unwrap();
        notifier
    }

    #[actix_rt::test]
    async fn success() {
        let notifier = get_notifier().await;
        for hook in Hook::iter() {
            let test_msg = uuid::Uuid::new_v4().to_string();
            notifier
                .send_message(
                    test_msg.clone(),
                    hook.clone(),
                    &FileInfo::new_test(),
                    &HeaderMap::new(),
                )
                .await
                .unwrap();
            let chan = notifier.channel_pool.get().await.unwrap();
            let message = chan
                .basic_get(
                    notifier.get_queue_name(hook).as_str(),
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
    }

    #[actix_rt::test]
    async fn unknown_url() {
        let notifier = AMQPNotifier::new(crate::config::AMQPHooksOptions {
            url: Some(String::from("http://unknown")),
            declare_exchange: true,
            declare_queues: true,
            durable_exchange: false,
            durable_queues: false,
            auto_delete: true,
            celery: false,
            exchange: uuid::Uuid::new_v4().to_string(),
            exchange_kind: String::from("topic"),
            routing_key: None,
            queues_prefix: uuid::Uuid::new_v4().to_string(),
            connection_pool_size: 1,
            channel_pool_size: 1,
            idle_connection_timeout: None,
            idle_channels_timeout: None,
        });
        let res = notifier
            .send_message(
                "Test Message".into(),
                Hook::PostCreate,
                &FileInfo::new_test(),
                &HeaderMap::new(),
            )
            .await;
        assert!(res.is_err());
    }
}
