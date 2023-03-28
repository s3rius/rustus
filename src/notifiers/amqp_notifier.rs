use crate::{
    config::AMQPHooksOptions,
    notifiers::{Hook, Notifier},
    RustusResult,
};
use actix_web::http::header::HeaderMap;
use async_trait::async_trait;
use bb8::Pool;
use bb8_lapin::LapinConnectionManager;
use lapin::{
    options::{BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable, LongString},
    BasicProperties, ChannelState, ConnectionProperties, ExchangeKind,
};
use std::time::Duration;
use strum::IntoEnumIterator;

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone)]
pub struct DeclareOptions {
    pub declare_exchange: bool,
    pub durable_exchange: bool,
    pub declare_queues: bool,
    pub durable_queues: bool,
}

#[derive(Clone)]
pub struct AMQPNotifier {
    exchange_name: String,
    channel_pool: Pool<ChannelPool>,
    queues_prefix: String,
    exchange_kind: String,
    routing_key: Option<String>,
    declare_options: DeclareOptions,
    celery: bool,
}

/// Channel manager for lapin channels.
///
/// This manager is used with bb8 pool,
/// so it maintains opened channels for every connections.
///
/// This pool uses connection pool
/// and issues new connections from it.
#[derive(Clone)]
pub struct ChannelPool {
    pool: Pool<LapinConnectionManager>,
}

impl ChannelPool {
    pub fn new(pool: Pool<LapinConnectionManager>) -> Self {
        ChannelPool { pool }
    }
}

/// ManagerConnection for ChannelPool.
///
/// This manager helps you maintain opened channels.
#[async_trait::async_trait]
impl bb8::ManageConnection for ChannelPool {
    type Connection = lapin::Channel;
    type Error = lapin::Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        Ok(self
            .pool
            .get()
            .await
            .map_err(|err| match err {
                bb8::RunError::TimedOut => lapin::Error::ChannelsLimitReached,
                bb8::RunError::User(user_err) => user_err,
            })?
            .create_channel()
            .await?)
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        let valid_states = vec![ChannelState::Initial, ChannelState::Connected];
        if valid_states.contains(&conn.status().state()) {
            Ok(())
        } else {
            Err(lapin::Error::InvalidChannel(conn.id()))
        }
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        let broken_states = vec![ChannelState::Closed, ChannelState::Error];
        broken_states.contains(&conn.status().state())
    }
}

impl AMQPNotifier {
    #[allow(clippy::fn_params_excessive_bools)]
    pub async fn new(options: AMQPHooksOptions) -> RustusResult<Self> {
        let manager = LapinConnectionManager::new(
            options.hooks_amqp_url.unwrap().as_str(),
            ConnectionProperties::default(),
        );
        let connection_pool = bb8::Pool::builder()
            .idle_timeout(
                options
                    .hooks_amqp_idle_connection_timeout
                    .map(Duration::from_secs),
            )
            .max_size(options.hooks_amqp_connection_pool_size)
            .build(manager)
            .await?;
        let channel_pool = bb8::Pool::builder()
            .idle_timeout(
                options
                    .hooks_amqp_idle_channels_timeout
                    .map(Duration::from_secs),
            )
            .max_size(options.hooks_amqp_channel_pool_size)
            .build(ChannelPool::new(connection_pool))
            .await?;

        Ok(Self {
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
        })
    }

    /// Generate queue name based on hook type.
    ///
    /// If specific routing key is not empty, it returns it.
    /// Otherwise it will generate queue name based on hook name.
    pub fn get_queue_name(&self, hook: Hook) -> String {
        if let Some(routing_key) = self.routing_key.as_ref() {
            routing_key.into()
        } else {
            format!("{}.{hook}", self.queues_prefix.as_str())
        }
    }
}

#[async_trait(?Send)]
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
                let queue_name = self.get_queue_name(hook);
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
        let chan = self.channel_pool.get().await?;
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
    use super::AMQPNotifier;
    use crate::notifiers::{amqp_notifier::DeclareOptions, Hook, Notifier};
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
        })
        .await
        .unwrap();
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
