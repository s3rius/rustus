use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    errors::{RustusError, RustusResult},
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
) -> RustusResult<HttpResponse> {
    let file_id_opt = request.match_info().get("file_id").map(String::from);
    if let Some(file_id) = file_id_opt {
        let file_info = state.info_storage.get_info(file_id.as_str()).await?;
        if file_info.storage != state.data_storage.to_string() {
            return Err(RustusError::FileNotFound);
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
            actix_web::rt::spawn(async move {
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
    use crate::{rustus_service, State};
    use actix_web::{
        http::StatusCode,
        test::{call_service, init_service, TestRequest},
        web, App,
    };
    use std::path::PathBuf;

    #[actix_rt::test]
    async fn success() {
        let state = State::test_new().await;
        let mut rustus = init_service(
            App::new().configure(rustus_service(web::Data::new(state.test_clone().await))),
        )
        .await;
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
        let mut rustus = init_service(
            App::new().configure(rustus_service(web::Data::new(state.test_clone().await))),
        )
        .await;
        let request = TestRequest::delete()
            .param("file_id", "not_exists")
            .to_request();
        let result = call_service(&mut rustus, request).await;
        assert_eq!(result.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn wrong_storage() {
        let state = State::test_new().await;
        let mut rustus = init_service(
            App::new().configure(rustus_service(web::Data::new(state.test_clone().await))),
        )
        .await;
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
