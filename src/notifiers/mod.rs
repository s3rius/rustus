pub mod base;
pub mod hooks;
pub mod impls;
pub mod manager;
pub mod serializer;

use axum::http::HeaderMap;
pub use manager::NotificationManager;
pub use serializer::Format;

#[derive(Clone, Debug)]
pub enum NotifierImpl {
    Http(impls::http_notifier::HttpNotifier),
    File(impls::file_notifier::FileNotifier),
    Dir(impls::dir_notifier::DirNotifier),
    Amqp(impls::amqp_notifier::AMQPNotifier),
}

impl base::Notifier for NotifierImpl {
    async fn prepare(&mut self) -> crate::errors::RustusResult<()> {
        match self {
            Self::Http(http) => http.prepare().await,
            Self::File(file) => file.prepare().await,
            Self::Dir(dir) => dir.prepare().await,
            Self::Amqp(amqp) => amqp.prepare().await,
        }
    }
    async fn send_message(
        &self,
        message: String,
        hook: hooks::Hook,
        headers_map: &HeaderMap,
    ) -> crate::errors::RustusResult<()> {
        match self {
            Self::Http(http) => http.send_message(message, hook, headers_map).await,
            Self::File(file) => file.send_message(message, hook, headers_map).await,
            Self::Dir(dir) => dir.send_message(message, hook, headers_map).await,
            Self::Amqp(amqp) => amqp.send_message(message, hook, headers_map).await,
        }
    }
}
