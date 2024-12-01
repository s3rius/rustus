use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    data_storage::base::DataStorage,
    errors::{RustusError, RustusResult},
    info_storage::base::InfoStorage,
    metrics,
    notifiers::Hook,
    State,
};

/// Terminate uploading.
///
/// This method will remove all data by id.
/// It removes info and actual data.
pub async fn terminate(
    request: HttpRequest,
    state: web::Data<State>,
    metrics: web::Data<metrics::RustusMetrics>,
) -> RustusResult<HttpResponse> {
    let file_id_opt = request.match_info().get("file_id").map(String::from);
    if let Some(file_id) = file_id_opt {
        let file_info = state.info_storage.get_info(file_id.as_str()).await?;
        if file_info.storage != state.data_storage.get_name() {
            return Err(RustusError::FileNotFound);
        }
        if state.config.hook_is_active(Hook::PreTerminate) {
            let message = state.config.notification_opts.hooks_format.format(
                &request,
                &file_info,
                state.config.notification_opts.behind_proxy,
            );
            let headers = request.headers();
            state
                .notification_manager
                .send_message(message, Hook::PreTerminate, headers)
                .await?;
        }
        state.info_storage.remove_info(file_id.as_str()).await?;
        state.data_storage.remove_file(&file_info).await?;
        metrics.terminated_uploads.inc();
        if state.config.hook_is_active(Hook::PostTerminate) {
            let message = state.config.notification_opts.hooks_format.format(
                &request,
                &file_info,
                state.config.notification_opts.behind_proxy,
            );
            let headers = request.headers().clone();
            tokio::task::spawn_local(async move {
                state
                    .notification_manager
                    .send_message(message, Hook::PostTerminate, &headers)
                    .await
            });
        }
    }
    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use crate::{info_storage::base::InfoStorage, server::test::get_service, State};
    use actix_web::{
        http::StatusCode,
        test::{call_service, TestRequest},
    };
    use std::path::PathBuf;

    #[actix_rt::test]
    async fn success() {
        let state = State::test_new().await;
        let mut rustus = get_service(state.clone()).await;
        let file_info = state.create_test_file().await;
        let request = TestRequest::delete()
            .uri(state.config.file_url(file_info.id.as_str()).as_str())
            .to_request();
        let response = call_service(&mut rustus, request).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(state
            .info_storage
            .get_info(file_info.id.as_str())
            .await
            .is_err());
        assert!(!PathBuf::from(file_info.path.unwrap()).exists());
    }

    #[actix_rt::test]
    async fn unknown_file_id() {
        let state = State::test_new().await;
        let mut rustus = get_service(state.clone()).await;
        let request = TestRequest::delete()
            .param("file_id", "not_exists")
            .to_request();
        let result = call_service(&mut rustus, request).await;
        assert_eq!(result.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn wrong_storage() {
        let state = State::test_new().await;
        let mut rustus = get_service(state.clone()).await;
        let mut file_info = state.create_test_file().await;
        file_info.storage = "unknown_storage".into();
        state
            .info_storage
            .set_info(&file_info, false)
            .await
            .unwrap();
        let request = TestRequest::delete()
            .uri(state.config.file_url(file_info.id.as_str()).as_str())
            .to_request();
        let response = call_service(&mut rustus, request).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
