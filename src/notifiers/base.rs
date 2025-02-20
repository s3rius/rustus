use crate::{errors::RustusResult, file_info::FileInfo};
use actix_web::http::header::HeaderMap;

use crate::notifiers::hooks::Hook;

pub trait Notifier {
    async fn prepare(&mut self) -> RustusResult<()>;
    async fn send_message(
        &self,
        message: String,
        hook: Hook,
        file_info: &FileInfo,
        headers_map: &HeaderMap,
    ) -> RustusResult<()>;
}
