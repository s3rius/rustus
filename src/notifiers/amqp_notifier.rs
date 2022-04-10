use crate::{
    notifiers::{Hook, Notifier},
    RustusResult,
};
use actix_web::http::header::HeaderMap;
use async_trait::async_trait;
use lapin::{
    options::{BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable, LongString},
    BasicProperties, ConnectionProperties, ExchangeKind,
};
use mobc_lapin::{mobc::Pool, RMQConnectionManager};
use strum::IntoEnumIterator;
use tokio_amqp::LapinTokioExt;

#[allow(clippy::struct_excessive_bools)]
pub struct DeclareOptions {
    pub declare_exchange: bool,
    pub durable_exchange: bool,
    pub declare_queues: bool,
    pub durable_queues: bool,
}

pub struct AMQPNotifier {
    exchange_name: String,
    pool: Pool<RMQConnectionManager>,
    queues_prefix: String,
    exchange_kind: String,
    routing_key: Option<String>,
    declare_options: DeclareOptions,
    celery: bool,
}

impl AMQPNotifier {
    #[allow(clippy::fn_params_excessive_bools)]
    pub fn new(
        amqp_url: &str,
        exchange: &str,
        queues_prefix: &str,
        exchange_kind: &str,
        routing_key: Option<String>,
        declare_options: DeclareOptions,
        celery: bool,
    ) -> Self {
        let manager = RMQConnectionManager::new(
            amqp_url.into(),
            ConnectionProperties::default().with_tokio(),
        );
        let pool = Pool::<RMQConnectionManager>::builder().build(manager);
        Self {
            pool,
            celery,
            routing_key,
            declare_options,
            exchange_kind: exchange_kind.into(),
            exchange_name: exchange.into(),
            queues_prefix: queues_prefix.into(),
        }
    }

    /// Generate queue name based on hook type.
    ///
    /// If specific routing key is not empty, it returns it.
    /// Otherwise it will generate queue name based on hook name.
    pub fn get_queue_name(&self, hook: Hook) -> String {
        if let Some(routing_key) = self.routing_key.as_ref() {
            routing_key.into()
        } else {
            format!("{}.{}", self.queues_prefix.as_str(), hook)
        }
    }
}

#[async_trait]
impl Notifier for AMQPNotifier {
    async fn prepare(&mut self) -> RustusResult<()> {
        let chan = self.pool.get().await?.create_channel().await?;
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
        let chan = self.pool.get().await?.create_channel().await?;
        let queue = self.get_queue_name(hook);
        let routing_key = self.routing_key.as_ref().unwrap_or(&queue);
        let payload = if self.celery {
            format!("[[{}], {{}}, {{}}]", message).as_bytes().to_vec()
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
                AMQPValue::LongString(LongString::from(format!("rustus.{}", hook))),
            );
        }
        chan.basic_publish(
            self.exchange_name.as_str(),
            routing_key.as_str(),
            BasicPublishOptions::default(),
            payload,
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
        let mut notifier = AMQPNotifier::new(
            amqp_url.as_str(),
            uuid::Uuid::new_v4().to_string().as_str(),
            uuid::Uuid::new_v4().to_string().as_str(),
            "topic",
            None,
            DeclareOptions {
                declare_exchange: true,
                declare_queues: true,
                durable_queues: false,
                durable_exchange: false,
            },
            true,
        );
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
        let chan = notifier
            .pool
            .get()
            .await
            .unwrap()
            .create_channel()
            .await
            .unwrap();
        let message = chan
            .basic_get(
                format!("{}.{}", notifier.queues_prefix.as_str(), hook).as_str(),
                BasicGetOptions::default(),
            )
            .await
            .unwrap();
        assert!(message.is_some());
        assert_eq!(
            String::from_utf8(message.clone().unwrap().data.clone()).unwrap(),
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
        let notifier = AMQPNotifier::new(
            "http://unknown",
            "test",
            "test",
            "topic",
            None,
            DeclareOptions {
                declare_exchange: false,
                declare_queues: false,
                durable_queues: false,
                durable_exchange: false,
            },
            false,
        );
        let res = notifier
            .send_message("Test Message".into(), Hook::PostCreate, &HeaderMap::new())
            .await;
        assert!(res.is_err());
    }
}
