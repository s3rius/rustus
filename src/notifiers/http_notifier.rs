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
    #[cfg_attr(coverage, no_coverage)]
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

#[cfg(test)]
mod tests {
    use super::HttpNotifier;
    use crate::notifiers::{Hook, Notifier};
    use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
    use httptest::matchers::contains;
    use httptest::responders::status_code;
    use std::str::FromStr;
    use std::time::Duration;

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

        let notifier = HttpNotifier::new(vec![hook_url], vec![]);
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

        let notifier = HttpNotifier::new(vec![hook_url], vec![]);
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

        let notifier = HttpNotifier::new(vec![hook_url], vec![]);
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
        let notifier = HttpNotifier::new(vec![hook_url], vec!["X-TEST-HEADER".into()]);
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
