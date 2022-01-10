use actix_web::{web, HttpRequest, HttpResponse};

use crate::errors::RustusResult;
use crate::notifiers::Hook;
use crate::State;

/// Terminate uploading.
///
/// This method will remove all data by id.
/// It removes info and actual data.
pub async fn terminate(
    request: HttpRequest,
    state: web::Data<State>,
) -> RustusResult<HttpResponse> {
    let file_id_opt = request.match_info().get("file_id").map(String::from);
    if let Some(file_id) = file_id_opt {
        let file_info = state.info_storage.get_info(file_id.as_str()).await?;
        if file_info.storage != state.data_storage.to_string() {
            return Ok(HttpResponse::NotFound().body(""));
        }
        state.info_storage.remove_info(file_id.as_str()).await?;
        state.data_storage.remove_file(&file_info).await?;
        if state.config.hook_is_active(Hook::PostTerminate) {
            let message = state
                .config
                .notification_opts
                .hooks_format
                .format(&request, &file_info)?;
            let headers = request.headers().clone();
            tokio::spawn(async move {
                state
                    .notification_manager
                    .send_message(message, Hook::PostTerminate, &headers)
                    .await
            });
        }
    }
    Ok(HttpResponse::NoContent().body(""))
}
