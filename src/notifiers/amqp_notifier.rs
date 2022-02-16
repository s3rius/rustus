use crate::notifiers::{Hook, Notifier};
use crate::RustusResult;
use actix_web::http::header::HeaderMap;
use async_trait::async_trait;
use lapin::options::{
    BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ConnectionProperties, ExchangeKind};
use mobc_lapin::mobc::Pool;
use mobc_lapin::RMQConnectionManager;
use strum::IntoEnumIterator;
use tokio_amqp::LapinTokioExt;

pub struct AMQPNotifier {
    exchange_name: String,
    pool: Pool<RMQConnectionManager>,
    queues_prefix: String,
}

impl AMQPNotifier {
    pub fn new(amqp_url: &str, exchange: &str, queues_prefix: &str) -> Self {
        let manager = RMQConnectionManager::new(
            amqp_url.into(),
            ConnectionProperties::default().with_tokio(),
        );
        let pool = Pool::<RMQConnectionManager>::builder().build(manager);
        Self {
            pool,
            exchange_name: exchange.into(),
            queues_prefix: queues_prefix.into(),
        }
    }

    pub fn get_queue_name(&self, hook: Hook) -> String {
        format!("{}.{}", self.queues_prefix.as_str(), hook)
    }
}

#[async_trait]
impl Notifier for AMQPNotifier {
    async fn prepare(&mut self) -> RustusResult<()> {
        let chan = self.pool.get().await?.create_channel().await?;
        chan.exchange_declare(
            self.exchange_name.as_str(),
            ExchangeKind::Topic,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;
        for hook in Hook::iter() {
            let queue_name = self.get_queue_name(hook);
            chan.queue_declare(
                queue_name.as_str(),
                QueueDeclareOptions::default(),
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
        chan.basic_publish(
            self.exchange_name.as_str(),
            queue.as_str(),
            BasicPublishOptions::default(),
            message.as_bytes().to_vec(),
            BasicProperties::default().with_content_type("application/json".into()),
        )
        .await?;
        Ok(())
    }
}

#[cfg(feature = "test_rmq")]
#[cfg(test)]
mod tests {
    use super::AMQPNotifier;
    use crate::notifiers::{Hook, Notifier};
    use actix_web::http::header::HeaderMap;
    use lapin::options::{BasicAckOptions, BasicGetOptions};

    async fn get_notifier() -> AMQPNotifier {
        let amqp_url = std::env::var("TEST_AMQP_URL").unwrap();
        let mut notifier = AMQPNotifier::new(
            amqp_url.as_str(),
            uuid::Uuid::new_v4().to_string().as_str(),
            uuid::Uuid::new_v4().to_string().as_str(),
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
            test_msg
        );
        message
            .unwrap()
            .ack(BasicAckOptions::default())
            .await
            .unwrap();
    }

    #[actix_rt::test]
    async fn unknown_url() {
        let notifier = AMQPNotifier::new("http://unknown", "test", "test");
        let res = notifier
            .send_message("Test Message".into(), Hook::PostCreate, &HeaderMap::new())
            .await;
        assert!(res.is_err());
    }
}
