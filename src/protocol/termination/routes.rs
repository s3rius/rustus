use actix_web::{web, HttpRequest, HttpResponse};

use crate::errors::RustusResult;
use crate::notifiers::Hook;
use crate::{InfoStorage, NotificationManager, RustusConf, Storage};

/// Terminate uploading.
///
/// This method will remove all data by id.
/// It removes info and actual data.
pub async fn terminate(
    storage: web::Data<Box<dyn Storage + Send + Sync>>,
    info_storage: web::Data<Box<dyn InfoStorage + Send + Sync>>,
    request: HttpRequest,
    notification_manager: web::Data<Box<NotificationManager>>,
    app_conf: web::Data<RustusConf>,
) -> RustusResult<HttpResponse> {
    let file_id_opt = request.match_info().get("file_id").map(String::from);
    if let Some(file_id) = file_id_opt {
        let file_info = info_storage.get_info(file_id.as_str()).await?;
        if file_info.storage != storage.to_string() {
            return Ok(HttpResponse::NotFound().body(""));
        }
        info_storage.remove_info(file_id.as_str()).await?;
        storage.remove_file(&file_info).await?;
        if app_conf.hook_is_active(Hook::PostTerminate) {
            let message = app_conf
                .notification_opts
                .hooks_format
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
