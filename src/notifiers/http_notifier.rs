use crate::errors::RustusResult;

use crate::notifiers::{Hook, Notifier};

use async_trait::async_trait;
use futures::future::try_join_all;
use log::debug;
use reqwest::Client;
use std::time::Duration;

pub struct HttpNotifier {
    urls: Vec<String>,
    client: Client,
}

impl HttpNotifier {
    pub fn new(urls: Vec<String>) -> Self {
        let client = Client::new();
        Self { urls, client }
    }
}

#[async_trait]
impl Notifier for HttpNotifier {
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    async fn send_message(&self, message: String, hook: Hook) -> RustusResult<()> {
        debug!("Starting HTTP Hook.");
        let idempotency_key = uuid::Uuid::new_v4().to_string();
        let requests_vec = self.urls.iter().map(|url| {
            debug!("Preparing request for {}", url);
            self.client
                .post(url.as_str())
                .header("Idempotency-Key", idempotency_key.as_str())
                .header("Hook-Name", hook.clone().to_string())
                .timeout(Duration::from_secs(2))
                .body(message.clone())
                .send()
        });
        let responses = try_join_all(requests_vec).await?;
        for resp in responses {
            resp.error_for_status()?;
        }
        Ok(())
    }
}
