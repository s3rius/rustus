use actix_web::{web, HttpRequest, HttpResponse};

use crate::errors::RustusError;

use crate::{RustusResult, State};

pub async fn get_file_info(
    state: web::Data<State>,
    request: HttpRequest,
) -> RustusResult<HttpResponse> {
    // Getting file id from URL.
    if request.match_info().get("file_id").is_none() {
        return Err(RustusError::FileNotFound);
    }
    let file_id = request.match_info().get("file_id").unwrap();

    // Getting file info from info_storage.
    let file_info = state.info_storage.get_info(file_id).await?;
    if file_info.storage != state.data_storage.to_string() {
        return Err(RustusError::FileNotFound);
    }
    let mut builder = HttpResponse::Ok();
    if file_info.is_partial {
        builder.insert_header(("Upload-Concat", "partial"));
    }
    if file_info.is_final && file_info.parts.is_some() {
        #[allow(clippy::or_fun_call)]
        let parts = file_info
            .parts
            .clone()
            .unwrap()
            .iter()
            .map(|file| {
                format!(
                    "{}/{}",
                    state
                        .config
                        .base_url()
                        .strip_suffix('/')
                        .unwrap_or(state.config.base_url().as_str()),
                    file.as_str()
                )
            })
            .collect::<Vec<String>>()
            .join(" ");
        builder.insert_header(("Upload-Concat", format!("final; {}", parts)));
    }
    builder
        .no_chunking(file_info.offset as u64)
        .insert_header(("Upload-Offset", file_info.offset.to_string()))
        .insert_header(("Content-Length", file_info.offset.to_string()));
    // Upload length is known.
    if let Some(upload_len) = file_info.length {
        builder.insert_header(("Upload-Length", upload_len.to_string()));
    } else {
        builder.insert_header(("Upload-Defer-Length", "1"));
    }
    if let Some(meta) = file_info.get_metadata_string() {
        builder.insert_header(("Upload-Metadata", meta));
    }
    Ok(builder.finish())
}

#[cfg(test)]
mod tests {
    use actix_web::http::{Method, StatusCode};

    use crate::{rustus_service, State};
    use actix_web::test::{call_service, init_service, TestRequest};
    use actix_web::{web, App};

    #[actix_rt::test]
    async fn success() {
        let state = State::test_new().await;
        let mut rustus = init_service(
            App::new().configure(rustus_service(web::Data::new(state.test_clone().await))),
        )
        .await;
        let mut file_info = state.create_test_file().await;
        file_info.offset = 100;
        file_info.length = Some(100);
        state
            .info_storage
            .set_info(&file_info, false)
            .await
            .unwrap();
        let request = TestRequest::with_uri(state.config.file_url(file_info.id.as_str()).as_str())
            .method(Method::HEAD)
            .to_request();
        let response = call_service(&mut rustus, request).await;
        let offset = response
            .headers()
            .get("Upload-Offset")
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        assert_eq!(file_info.offset, offset)
    }

    #[actix_rt::test]
    async fn success_metadata() {
        let state = State::test_new().await;
        let mut rustus = init_service(
            App::new().configure(rustus_service(web::Data::new(state.test_clone().await))),
        )
        .await;
        let mut file_info = state.create_test_file().await;
        file_info.offset = 100;
        file_info.length = Some(100);
        file_info.metadata.insert("test".into(), "value".into());
        state
            .info_storage
            .set_info(&file_info, false)
            .await
            .unwrap();
        let request = TestRequest::with_uri(state.config.file_url(file_info.id.as_str()).as_str())
            .method(Method::HEAD)
            .to_request();
        let response = call_service(&mut rustus, request).await;
        let metadata = response
            .headers()
            .get("Upload-Metadata")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(
            String::from(metadata),
            format!("{} {}", "test", base64::encode("value"))
        )
    }

    #[actix_rt::test]
    async fn success_defer_len() {
        let state = State::test_new().await;
        let mut rustus = init_service(
            App::new().configure(rustus_service(web::Data::new(state.test_clone().await))),
        )
        .await;
        let mut file_info = state.create_test_file().await;
        file_info.deferred_size = true;
        file_info.length = None;
        state
            .info_storage
            .set_info(&file_info, false)
            .await
            .unwrap();
        let request = TestRequest::with_uri(state.config.file_url(file_info.id.as_str()).as_str())
            .method(Method::HEAD)
            .to_request();
        let response = call_service(&mut rustus, request).await;
        assert_eq!(
            response
                .headers()
                .get("Upload-Defer-Length")
                .unwrap()
                .to_str()
                .unwrap(),
            "1"
        );
    }

    #[actix_rt::test]
    async fn test_get_file_info_partial() {
        let state = State::test_new().await;
        let mut rustus = init_service(
            App::new().configure(rustus_service(web::Data::new(state.test_clone().await))),
        )
        .await;
        let mut file_info = state.create_test_file().await;
        file_info.is_partial = true;
        state
            .info_storage
            .set_info(&file_info, false)
            .await
            .unwrap();
        let request = TestRequest::with_uri(state.config.file_url(file_info.id.as_str()).as_str())
            .method(Method::HEAD)
            .to_request();
        let response = call_service(&mut rustus, request).await;
        assert_eq!(
            response
                .headers()
                .get("Upload-Concat")
                .unwrap()
                .to_str()
                .unwrap(),
            "partial"
        );
    }

    #[actix_rt::test]
    async fn success_final() {
        let state = State::test_new().await;
        let mut rustus = init_service(
            App::new().configure(rustus_service(web::Data::new(state.test_clone().await))),
        )
        .await;
        let mut file_info = state.create_test_file().await;
        file_info.is_partial = false;
        file_info.is_final = true;
        file_info.parts = Some(vec!["test1".into(), "test2".into()]);
        state
            .info_storage
            .set_info(&file_info, false)
            .await
            .unwrap();
        let request = TestRequest::with_uri(state.config.file_url(file_info.id.as_str()).as_str())
            .method(Method::HEAD)
            .to_request();
        let response = call_service(&mut rustus, request).await;
        assert_eq!(
            response
                .headers()
                .get("Upload-Concat")
                .unwrap()
                .to_str()
                .unwrap(),
            format!(
                "final; {} {}",
                state.config.file_url("test1"),
                state.config.file_url("test2")
            )
            .as_str()
        );
    }

    #[actix_rt::test]
    async fn no_file() {
        let state = State::test_new().await;
        let mut rustus = init_service(
            App::new().configure(rustus_service(web::Data::new(state.test_clone().await))),
        )
        .await;
        let request = TestRequest::with_uri(state.config.file_url("unknknown").as_str())
            .method(Method::HEAD)
            .to_request();
        let response = call_service(&mut rustus, request).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn test_get_file_info_wrong_storage() {
        let state = State::test_new().await;
        let mut rustus = init_service(
            App::new().configure(rustus_service(web::Data::new(state.test_clone().await))),
        )
        .await;
        let mut file_info = state.create_test_file().await;
        file_info.storage = String::from("unknown");
        state
            .info_storage
            .set_info(&file_info, false)
            .await
            .unwrap();
        let request = TestRequest::with_uri(state.config.file_url(file_info.id.as_str()).as_str())
            .method(Method::HEAD)
            .to_request();
        let response = call_service(&mut rustus, request).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
