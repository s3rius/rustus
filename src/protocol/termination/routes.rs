use actix_web::{web, HttpRequest, HttpResponse};

use crate::errors::RustusResult;
use crate::notifiers::Hook;
use crate::{NotificationManager, RustusConf, Storage};

/// Terminate uploading.
///
/// This method will remove all
/// files by id.
pub async fn terminate(
    storage: web::Data<Box<dyn Storage + Send + Sync>>,
    request: HttpRequest,
    notification_manager: web::Data<Box<NotificationManager>>,
    app_conf: web::Data<RustusConf>,
) -> RustusResult<HttpResponse> {
    let file_id_opt = request.match_info().get("file_id").map(String::from);
    if let Some(file_id) = file_id_opt {
        let file_info = storage.remove_file(file_id.as_str()).await?;
        if app_conf.hook_is_active(Hook::PostTerminate) {
            let message = app_conf
                .notification_opts
                .notification_format
                .format(&request, &file_info)?;
            let headers = request.headers().clone();
            tokio::spawn(async move {
                notification_manager
                    .send_message(message, Hook::PostTerminate, &headers)
                    .await
            });
        }
    }
    Ok(HttpResponse::NoContent().body(""))
}
