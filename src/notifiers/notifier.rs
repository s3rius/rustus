use async_trait::async_trait;

#[async_trait]
trait Notifier {
    async fn send_message();
}
