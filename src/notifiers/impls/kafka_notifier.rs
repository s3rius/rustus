use rdkafka::config::FromClientConfig;
use rdkafka::producer::FutureProducer;
use rdkafka::producer::FutureRecord;
use rdkafka::util::Timeout;
use rdkafka::ClientConfig;
use std::collections::HashMap;
use std::time::Duration;

use actix_web::http::header::HeaderMap;
use std::str::FromStr;

use crate::errors::RustusError;
use crate::errors::RustusResult;
use crate::file_info::FileInfo;
use crate::notifiers::base::Notifier;

#[derive(Debug, Clone)]
pub struct ExtraKafkaOptions {
    opts: HashMap<String, String>,
}

impl FromStr for ExtraKafkaOptions {
    type Err = RustusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut opts = HashMap::new();
        for opt in s.split(';') {
            let mut parts = opt.split('=');
            let key = parts.next().ok_or_else(|| {
                RustusError::KafkaExtraOptionsError(String::from(
                    "Cannot read option name before `=` sign",
                ))
            })?;
            let value = parts.next().ok_or_else(|| {
                RustusError::KafkaExtraOptionsError(String::from(
                    "Cannot read value after `=` sign.",
                ))
            })?;
            opts.insert(key.to_string(), value.to_string());
        }
        Ok(Self { opts })
    }
}

impl ExtraKafkaOptions {
    pub fn fill_config(&self, config: &mut ClientConfig) {
        for (key, value) in &self.opts {
            config.set(key, value);
        }
    }
}

#[derive(Clone)]
pub struct KafkaNotifier {
    producer: FutureProducer,
    topic: Option<String>,
    prefix: Option<String>,
    send_timeout: Timeout,
}

impl KafkaNotifier {
    pub fn new(
        hosts: String,
        client_id: Option<String>,
        topic: Option<String>,
        prefix: Option<String>,
        requred_acks: Option<String>,
        compression: Option<String>,
        idle_timeout: Option<u64>,
        send_timeout: Option<u64>,
        extra_opts: Option<ExtraKafkaOptions>,
    ) -> RustusResult<Self> {
        let mut config = ClientConfig::new();

        config.set("bootstrap.servers", hosts);

        if let Some(client_id) = client_id {
            config.set("client.id", client_id);
        }
        if let Some(acks) = requred_acks {
            config.set("request.required.acks", acks.to_string());
        }

        if let Some(connection_timeout) = idle_timeout {
            config.set("connections.max.idle.ms", connection_timeout.to_string());
        }

        if let Some(compression) = compression {
            config.set("compression.codec", compression.to_string());
        }

        if let Some(extra_options) = extra_opts {
            extra_options.fill_config(&mut config);
        }

        let send_timeout = Timeout::from(send_timeout.map(Duration::from_secs));

        let producer = FutureProducer::from_config(&config)?;
        Ok(KafkaNotifier {
            producer,
            topic,
            prefix,
            send_timeout,
        })
    }
}

impl Notifier for KafkaNotifier {
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    async fn send_message(
        &self,
        message: String,
        hook: crate::notifiers::Hook,
        file_info: &FileInfo,
        _headers_map: &HeaderMap,
    ) -> RustusResult<()> {
        let hook_name = hook.to_string();
        let topic = self.topic.as_ref().unwrap_or(&hook_name);
        let topic = match &self.prefix {
            Some(prefix) => format!("{}-{}", prefix, topic),
            None => topic.to_string(),
        };
        log::debug!(
            "Sending message to Kafka topic {topic} with a key {key}.",
            key = file_info.id
        );
        {
            let send_res = self
                .producer
                .send(
                    FutureRecord::to(topic.as_str())
                        .key(file_info.id.as_str())
                        .payload(&message),
                    self.send_timeout,
                )
                .await;
            if let Err((kafka_err, msg)) = send_res {
                log::debug!("Failed to send message to Kafka: {:#?}", msg);
                return Err(RustusError::KafkaError(kafka_err));
            }
            log::debug!("Sending a `{}` hook with body `{}`", hook, message);
            // self.producer.write().await.send(&Record::from_key_value(
            //     topic.as_str(),
            //     file_info.id.as_str(),
            //     message,
            // ))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::notifiers::{base::Notifier, Hook};

    use super::KafkaNotifier;
    use actix_web::http::header::HeaderMap;
    use kafka::consumer::Consumer;

    fn get_notifier(topic: Option<&str>, prefix: Option<&str>) -> KafkaNotifier {
        let urls = std::env::var("TEST_KAFKA_URLS")
            .unwrap_or(String::from("localhost:9092"))
            .split(',')
            .map(String::from)
            .collect::<Vec<_>>();
        KafkaNotifier::new(
            urls,
            Some("rustus".to_string()),
            topic.map(String::from),
            prefix.map(String::from),
            None,
            None,
            None,
            None,
        )
        .unwrap()
    }

    fn get_consumer(topics: Vec<&str>) -> Consumer {
        let urls = std::env::var("TEST_KAFKA_URLS")
            .unwrap_or(String::from("localhost:9092"))
            .split(',')
            .map(String::from)
            .collect::<Vec<_>>();
        let mut builder = Consumer::from_hosts(urls).with_group(String::from("rustus-test"));
        for topic in topics {
            builder = builder.with_topic(String::from(topic));
        }
        builder.create().unwrap()
    }

    #[actix_rt::test]
    async fn simple_success() {
        let notifier = get_notifier(None, None);
        let finfo = crate::file_info::FileInfo::new_test();
        let consumer = get_consumer(vec![&Hook::PreCreate.to_string()]);
        let data = String::from("data");
        notifier
            .send_message(data.clone(), Hook::PreCreate, &finfo, &HeaderMap::default())
            .await
            .unwrap();
    }
}
