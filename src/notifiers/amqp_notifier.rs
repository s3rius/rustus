use crate::notifiers::{Hook, Notifier};
use crate::{RustusConf, RustusResult};
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
}

impl AMQPNotifier {
    pub fn new(app_conf: RustusConf) -> Self {
        let manager = RMQConnectionManager::new(
            app_conf.notification_opts.hooks_amqp_url.unwrap(),
            ConnectionProperties::default().with_tokio(),
        );
        let pool = Pool::<RMQConnectionManager>::builder().build(manager);
        Self {
            pool,
            exchange_name: app_conf.notification_opts.hooks_amqp_exchange,
        }
    }

    pub fn get_queue_name(hook: Hook) -> String {
        format!("rustus.{}", hook)
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
            let queue_name = Self::get_queue_name(hook);
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

    async fn send_message(&self, message: String, hook: Hook) -> RustusResult<()> {
        let chan = self.pool.get().await?.create_channel().await?;
        let queue = Self::get_queue_name(hook);
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
