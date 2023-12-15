use crate::{
    errors::{RustusError, RustusResult},
    notifiers::{base::Notifier, hooks::Hook},
};

use axum::http::HeaderMap;
use reqwest::Client;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct HttpNotifier {
    urls: Vec<String>,
    client: Client,
    forward_headers: Vec<String>,
    timeout_secs: u64,
}

impl HttpNotifier {
    #[must_use]
    pub fn new(urls: Vec<String>, forward_headers: Vec<String>, timeout_secs: Option<u64>) -> Self {
        let client = Client::new();
        Self {
            urls,
            client,
            forward_headers,
            timeout_secs: timeout_secs.unwrap_or(2),
        }
    }
}

impl Notifier for HttpNotifier {
    #[cfg_attr(coverage, no_coverage)]
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    #[tracing::instrument(err, skip(self, message, header_map), fields(response_body = tracing::field::Empty))]
    async fn send_message(
        &self,
        message: &str,
        hook: &Hook,
        header_map: &HeaderMap,
    ) -> RustusResult<()> {
        tracing::debug!("Starting HTTP Hook.");
        let idempotency_key = uuid::Uuid::new_v4().to_string();
        let body_bytes = bytes::Bytes::copy_from_slice(message.as_bytes());
        let requests_vec = self.urls.iter().map(|url| {
            tracing::debug!("Preparing request for {}", url);
            let mut request = self
                .client
                .post(url.as_str())
                .header("Idempotency-Key", idempotency_key.as_str())
                .header("Hook-Name", hook.to_string())
                .header("Content-Type", "application/json")
                .timeout(Duration::from_secs(self.timeout_secs));
            for item in &self.forward_headers {
                if let Some(value) = header_map.get(item.as_str()) {
                    request = request.header(item.as_str(), value.as_bytes());
                }
            }
            request.body(body_bytes.clone()).send()
        });
        for response in requests_vec {
            let real_resp = response.await?;
            if !real_resp.status().is_success() {
                let content_type = real_resp
                    .headers()
                    .get("Content-Type")
                    .and_then(|hval| hval.to_str().ok().map(String::from));
                let status = real_resp.status().as_u16();
                let text = real_resp.text().await.unwrap_or_default();
                tracing::Span::current().record("response_body", &text);
                return Err(RustusError::HTTPHookError(status, text, content_type));
            }
        }
        Ok(())
    }
}
