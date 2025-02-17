use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    data_storage::base::DataStorage, errors::RustusError, info_storage::base::InfoStorage,
    RustusResult, State,
};

/// Retrieve actual file.
///
/// This method allows you to download files directly from storage.
pub async fn get_file(request: HttpRequest, state: web::Data<State>) -> RustusResult<HttpResponse> {
    let file_id_opt = request.match_info().get("file_id").map(String::from);
    if let Some(file_id) = file_id_opt {
        let file_info = state.info_storage.get_info(file_id.as_str()).await?;
        if file_info.storage != state.data_storage.get_name() {
            return Err(RustusError::FileNotFound);
        }
        state.data_storage.get_contents(&file_info, &request).await
    } else {
        Err(RustusError::FileNotFound)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        data_storage::base::DataStorage, info_storage::base::InfoStorage,
        server::test::get_service, State,
    };
    use actix_web::{
        http::StatusCode,
        test::{call_service, TestRequest},
    };
    use bytes::Bytes;

    #[actix_rt::test]
    async fn success() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let mut file_info = state.create_test_file().await;
        state
            .data_storage
            .add_bytes(&mut file_info, Bytes::from("testing"))
            .await
            .unwrap();
        let request = TestRequest::get()
            .uri(state.config.file_url(file_info.id.as_str()).as_str())
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert!(resp.status().is_success());
    }

    #[actix_rt::test]
    async fn unknown_file_id() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let request = TestRequest::get()
            .uri(state.config.file_url("random_str").as_str())
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn unknown_storage() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let mut file_info = state.create_test_file().await;
        file_info.storage = "unknown_storage".into();
        state
            .info_storage
            .set_info(&file_info, false)
            .await
            .unwrap();
        let request = TestRequest::get()
            .uri(state.config.file_url(file_info.id.as_str()).as_str())
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
