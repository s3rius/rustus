use crate::errors::{RustusError, RustusResult};

use crate::notifiers::{Hook, Notifier};

use actix_web::http::header::HeaderMap;
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct HttpNotifier {
    urls: Vec<String>,
    client: Client,
    forward_headers: Vec<String>,
    timeout_secs: u64,
}

impl HttpNotifier {
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

#[async_trait(?Send)]
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
                .header("Hook-Name", hook.to_string())
                .header("Content-Type", "application/json")
                .timeout(Duration::from_secs(self.timeout_secs));
            for item in &self.forward_headers {
                if let Some(value) = header_map.get(item.as_str()) {
                    request = request.header(item.as_str(), value.as_bytes());
                }
            }
            request.body(message.clone()).send()
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
                log::warn!(
                    "Got wrong response for `{hook}`. Status code: `{status}`, body: `{body}`",
                    hook = hook,
                    status = status,
                    body = text,
                );
                return Err(RustusError::HTTPHookError(status, text, content_type));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::HttpNotifier;
    use crate::notifiers::{Hook, Notifier};
    use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
    use httptest::{matchers::contains, responders::status_code};
    use std::{str::FromStr, time::Duration};

    #[actix_rt::test]
    async fn success_request() {
        let server = httptest::Server::run();
        server.expect(
            httptest::Expectation::matching(httptest::matchers::request::method_path(
                "POST", "/hook",
            ))
            .respond_with(httptest::responders::status_code(200)),
        );
        let hook_url = server.url_str("/hook");

        let notifier = HttpNotifier::new(vec![hook_url], vec![], None);
        notifier
            .send_message("test_message".into(), Hook::PostCreate, &HeaderMap::new())
            .await
            .unwrap();
    }

    #[actix_rt::test]
    async fn timeout_request() {
        let server = httptest::Server::run();
        server.expect(
            httptest::Expectation::matching(httptest::matchers::request::method_path(
                "POST", "/hook",
            ))
            .respond_with(httptest::responders::delay_and_then(
                Duration::from_secs(3),
                status_code(200),
            )),
        );
        let hook_url = server.url_str("/hook");

        let notifier = HttpNotifier::new(vec![hook_url], vec![], None);
        let result = notifier
            .send_message("test_message".into(), Hook::PostCreate, &HeaderMap::new())
            .await;
        assert!(result.is_err());
    }

    #[actix_rt::test]
    async fn unknown_url() {
        let server = httptest::Server::run();
        server.expect(
            httptest::Expectation::matching(httptest::matchers::request::method_path(
                "POST", "/hook",
            ))
            .respond_with(httptest::responders::status_code(404)),
        );
        let hook_url = server.url_str("/hook");

        let notifier = HttpNotifier::new(vec![hook_url], vec![], None);
        let result = notifier
            .send_message("test_message".into(), Hook::PostCreate, &HeaderMap::new())
            .await;
        assert!(result.is_err());
    }

    #[actix_rt::test]
    async fn forwarded_header() {
        let server = httptest::Server::run();
        server.expect(
            httptest::Expectation::matching(httptest::matchers::all_of![
                httptest::matchers::request::method_path("POST", "/hook",),
                httptest::matchers::request::headers(contains(("x-test-header", "meme-value")))
            ])
            .respond_with(httptest::responders::status_code(200)),
        );
        let hook_url = server.url_str("/hook");
        let notifier = HttpNotifier::new(vec![hook_url], vec!["X-TEST-HEADER".into()], None);
        let mut header_map = HeaderMap::new();
        header_map.insert(
            HeaderName::from_str("X-TEST-HEADER").unwrap(),
            HeaderValue::from_str("meme-value").unwrap(),
        );
        notifier
            .send_message("test_message".into(), Hook::PostCreate, &header_map)
            .await
            .unwrap();
    }
}
