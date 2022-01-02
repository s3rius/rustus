use crate::errors::RustusResult;

use crate::notifiers::{Hook, Notifier};

use actix_web::http::header::HeaderMap;
use async_trait::async_trait;
use futures::future::try_join_all;
use log::debug;
use reqwest::Client;
use std::time::Duration;

pub struct HttpNotifier {
    urls: Vec<String>,
    client: Client,
    forward_headers: Vec<String>,
}

impl HttpNotifier {
    pub fn new(urls: Vec<String>, forward_headers: Vec<String>) -> Self {
        let client = Client::new();
        Self {
            urls,
            client,
            forward_headers,
        }
    }
}

#[async_trait]
impl Notifier for HttpNotifier {
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    async fn send_message(
        &self,
        message: String,
        hook: Hook,
        header_map: &HeaderMap,
    ) -> RustusResult<()> {
        debug!("Starting HTTP Hook.");
        let idempotency_key = uuid::Uuid::new_v4().to_string();
        let requests_vec = self.urls.iter().map(|url| {
            debug!("Preparing request for {}", url);
            let mut request = self
                .client
                .post(url.as_str())
                .header("Idempotency-Key", idempotency_key.as_str())
                .header("Hook-Name", hook.clone().to_string())
                .timeout(Duration::from_secs(2));
            for item in &self.forward_headers {
                if let Some(value) = header_map.get(item.clone()) {
                    request = request.header(item.clone(), value.as_bytes());
                }
            }
            request.body(message.clone()).send()
        });
        let responses = try_join_all(requests_vec).await?;
        for resp in responses {
            resp.error_for_status()?;
        }
        Ok(())
    }
}
