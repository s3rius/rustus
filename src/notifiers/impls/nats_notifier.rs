use async_nats::{client::Client as NatsClient, ConnectOptions};

use crate::{
    errors::{RustusError, RustusResult},
    notifiers::base::Notifier,
};

#[derive(Debug, Clone)]
pub struct NatsNotifier {
    nats_client: NatsClient,
    subject: Option<String>,
    prefix: Option<String>,
    wait_for_replies: bool,
}

impl NatsNotifier {
    pub async fn new(
        urls: Vec<String>,
        subject: Option<String>,
        prefix: Option<String>,
        wait_for_replies: bool,
        username: Option<String>,
        password: Option<String>,
        token: Option<String>,
    ) -> RustusResult<Self> {
        let mut options = ConnectOptions::new();

        match (username, password) {
            (Some(user), Some(pass)) => options = options.user_and_password(user, pass),
            (None, None) => (),
            (_, _) => {
                return Err(RustusError::Unimplemented(String::from(
                    "Both username and password must be provided.",
                )))
            }
        }
        if let Some(token) = token {
            options = options.token(token);
        }

        let nats_client = options.connect(urls).await?;

        Ok(Self {
            nats_client,
            subject,
            prefix,
            wait_for_replies,
        })
    }
}

impl Notifier for NatsNotifier {
    async fn prepare(&mut self) -> RustusResult<()> {
        Ok(())
    }

    async fn send_message(
        &self,
        message: String,
        hook: crate::notifiers::Hook,
        _file_info: &crate::file_info::FileInfo,
        headers_map: &actix_web::http::header::HeaderMap,
    ) -> RustusResult<()> {
        let hook_name = hook.to_string();
        let subject = self.prefix.as_ref().map_or_else(
            || self.subject.as_ref().unwrap_or(&hook_name).to_owned(),
            |prefix| format!("{prefix}.{hook_name}"),
        );
        let mut headers = async_nats::HeaderMap::new();
        for (key, value) in headers_map {
            headers.insert(key.as_str(), value.to_str().unwrap());
        }
        log::debug!("Sending message to NATS subject {subject}.");
        if self.wait_for_replies {
            let response = self
                .nats_client
                .request_with_headers(subject, headers, message.into())
                .await?;
            log::debug!("Received NATS response: {:?}", response);
            if !(response.payload.is_empty() || *response.payload == *b"OK") {
                return Err(RustusError::NatsErrorResponse(
                    String::from_utf8_lossy(&response.payload).to_string(),
                ));
            }
        } else {
            self.nats_client
                .publish_with_headers(subject, headers, message.into())
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::notifiers::{base::Notifier, Hook};

    use super::NatsNotifier;
    use actix_web::http::header::HeaderMap;
    use bytes::Bytes;
    use futures::StreamExt;

    async fn get_notifier(
        subject: Option<&str>,
        prefix: Option<&str>,
        wait_replies: bool,
    ) -> NatsNotifier {
        let urls = std::env::var("TEST_NATS_URLS")
            .unwrap_or(String::from("localhost:4222"))
            .split(',')
            .map(String::from)
            .collect::<Vec<_>>();
        NatsNotifier::new(
            urls,
            subject.map(String::from),
            prefix.map(String::from),
            wait_replies,
            None,
            None,
            None,
        )
        .await
        .unwrap()
    }

    async fn get_client() -> async_nats::Client {
        let urls = std::env::var("TEST_NATS_URLS")
            .unwrap_or(String::from("localhost:4222"))
            .split(',')
            .map(String::from)
            .collect::<Vec<_>>();
        let client = async_nats::ConnectOptions::new()
            .connect(urls)
            .await
            .unwrap();
        client
    }

    #[actix_rt::test]
    async fn simple_success_on_subject() {
        let subject = uuid::Uuid::new_v4().simple().to_string();
        let notifier = get_notifier(Some(subject.as_str()), None, false).await;
        let finfo = crate::file_info::FileInfo::new_test();
        let client = get_client().await;
        let data = String::from("data");

        let cloned_subject = subject.clone();
        let cloned_client = client.clone();
        let (readiness_tx, readiness_rx) = tokio::sync::oneshot::channel();
        let listen_task = tokio::spawn(async move {
            let mut subscription = cloned_client.subscribe(cloned_subject).await.unwrap();
            readiness_tx.send(()).unwrap();
            subscription.next().await.unwrap()
        });
        readiness_rx.await.unwrap();
        notifier
            .send_message(data.clone(), Hook::PreCreate, &finfo, &HeaderMap::default())
            .await
            .unwrap();

        let msg = tokio::time::timeout(Duration::from_secs(1), listen_task)
            .await
            .unwrap()
            .unwrap();
        assert!(msg.reply.is_none());
        assert_eq!(msg.payload, data.as_bytes());
    }

    #[actix_rt::test]
    async fn simple_success_on_prefix() {
        let prefix = uuid::Uuid::new_v4().simple().to_string();
        let notifier = get_notifier(None, Some(&prefix), false).await;
        let finfo = crate::file_info::FileInfo::new_test();
        let client = get_client().await;
        let subject = format!("{prefix}.pre-create");
        let data = String::from("data");

        let cloned_subject = subject.clone();
        let cloned_client = client.clone();
        let (readiness_tx, readiness_rx) = tokio::sync::oneshot::channel();
        let listen_task = tokio::spawn(async move {
            let mut subscription = cloned_client.subscribe(cloned_subject).await.unwrap();
            readiness_tx.send(()).unwrap();
            subscription.next().await.unwrap()
        });

        readiness_rx.await.unwrap();
        notifier
            .send_message(data.clone(), Hook::PreCreate, &finfo, &HeaderMap::default())
            .await
            .unwrap();
        println!("Message sent");

        let msg = tokio::time::timeout(Duration::from_secs(1), listen_task)
            .await
            .unwrap()
            .unwrap();

        assert!(msg.reply.is_none());
        assert_eq!(msg.payload, data.as_bytes());
    }

    #[actix_rt::test]
    async fn simple_success_on_subj_wait_reply() {
        let subject = uuid::Uuid::new_v4().simple().to_string();
        let notifier = get_notifier(Some(subject.as_str()), None, true).await;
        let finfo = crate::file_info::FileInfo::new_test();
        let client = get_client().await;
        let data = String::from("data");
        let cloned_data = data.clone();

        let cloned_subject = subject.clone();
        let cloned_client = client.clone();
        let (readiness_tx, readiness_rx) = tokio::sync::oneshot::channel::<()>();
        let listen_task = tokio::spawn(async move {
            let mut subscription = cloned_client.subscribe(cloned_subject).await.unwrap();
            readiness_tx.send(()).unwrap();
            subscription.next().await.unwrap()
        });

        readiness_rx.await.unwrap();

        let sender_task = tokio::spawn(async move {
            notifier
                .send_message(cloned_data, Hook::PreCreate, &finfo, &HeaderMap::default())
                .await
                .unwrap()
        });
        let msg = tokio::time::timeout(Duration::from_secs(1), listen_task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(msg.payload, data.as_bytes());
        assert!(!sender_task.is_finished());
        client
            .publish(msg.reply.unwrap(), Bytes::from_static(b"OK"))
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), sender_task)
            .await
            .unwrap()
            .unwrap();
    }

    #[actix_rt::test]
    async fn simple_success_on_prefix_wait_reply() {
        let prefix = uuid::Uuid::new_v4().simple().to_string();
        let notifier = get_notifier(None, Some(&prefix), true).await;
        let finfo = crate::file_info::FileInfo::new_test();
        let client = get_client().await;
        let data = String::from("data");
        let cloned_data = data.clone();
        let hook = Hook::PreCreate;

        let subject = format!("{prefix}.{hook}");
        let cloned_client = client.clone();
        let (readiness_tx, readiness_rx) = tokio::sync::oneshot::channel::<()>();
        let listen_task = tokio::spawn(async move {
            let mut subscription = cloned_client.subscribe(subject).await.unwrap();
            readiness_tx.send(()).unwrap();
            subscription.next().await.unwrap()
        });

        readiness_rx.await.unwrap();

        let sender_task = tokio::spawn(async move {
            notifier
                .send_message(cloned_data, hook, &finfo, &HeaderMap::default())
                .await
                .unwrap()
        });
        let msg = tokio::time::timeout(Duration::from_secs(1), listen_task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(msg.payload, data.as_bytes());
        assert!(!sender_task.is_finished());
        client
            .publish(msg.reply.unwrap(), Bytes::from_static(b"OK"))
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), sender_task)
            .await
            .unwrap()
            .unwrap();
    }
}
