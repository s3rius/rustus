use actix_web::{
    http::header::{CacheControl, CacheDirective},
    web,
    web::Bytes,
    HttpRequest, HttpResponse,
};

#[cfg(feature = "hashers")]
use crate::utils::hashes::verify_chunk_checksum;
use crate::{
    errors::RustusError,
    notifiers::Hook,
    protocol::extensions::Extensions,
    utils::headers::{check_header, parse_header},
    RustusResult, State,
};

pub async fn write_bytes(
    request: HttpRequest,
    bytes: Bytes,
    state: web::Data<State>,
    #[cfg(feature = "metrics")] active_uploads: web::Data<prometheus::IntGauge>,
) -> RustusResult<HttpResponse> {
    // Checking if request has required headers.
    let check_content_type = |val: &str| val == "application/offset+octet-stream";
    if !check_header(&request, "Content-Type", check_content_type) {
        return Ok(HttpResponse::UnsupportedMediaType().body("Unknown content-type."));
    }
    // Getting current offset.
    let offset: Option<usize> = parse_header(&request, "Upload-Offset");

    if offset.is_none() {
        return Ok(HttpResponse::UnsupportedMediaType().body("No offset provided."));
    }

    if request.match_info().get("file_id").is_none() {
        return Err(RustusError::FileNotFound);
    }

    #[cfg(feature = "hashers")]
    if state.config.tus_extensions.contains(&Extensions::Checksum) {
        if let Some(header) = request.headers().get("Upload-Checksum").cloned() {
            let cloned_bytes = bytes.clone();
            if !tokio::task::spawn_blocking(move || {
                verify_chunk_checksum(&header, cloned_bytes.as_ref())
            })
            .await??
            {
                return Err(RustusError::WrongChecksum);
            }
        }
    }

    // New upload length.
    // Parses header `Upload-Length` only if the creation-defer-length extension is enabled.
    let updated_len = if state
        .config
        .tus_extensions
        .contains(&Extensions::CreationDeferLength)
    {
        parse_header(&request, "Upload-Length")
    } else {
        None
    };

    let file_id = request.match_info().get("file_id").unwrap();
    // Getting file info.
    let mut file_info = state.info_storage.get_info(file_id).await?;

    // According to TUS protocol you can't update final uploads.
    if file_info.is_final {
        return Ok(HttpResponse::Forbidden().finish());
    }

    // Checking if file was stored in the same storage.
    if file_info.storage != state.data_storage.to_string() {
        return Err(RustusError::FileNotFound);
    }
    // Checking if offset from request is the same as the real offset.
    if offset.unwrap() != file_info.offset {
        return Ok(HttpResponse::Conflict().finish());
    }

    // If someone want to update file length.
    // This required by Upload-Defer-Length extension.
    if let Some(new_len) = updated_len {
        // Whoop, someone gave us total file length
        // less that he had already uploaded.
        if new_len < file_info.offset {
            return Err(RustusError::WrongOffset);
        }
        // We already know the exact size of a file.
        // Someone want to update it.
        // Anyway, it's not allowed, heh.
        if file_info.length.is_some() {
            return Err(RustusError::SizeAlreadyKnown);
        }

        // All checks are ok. Now our file will have exact size.
        file_info.deferred_size = false;
        file_info.length = Some(new_len);
    }

    // Checking if the size of the upload is already equals
    // to calculated offset. It means that all bytes were already written.
    if Some(file_info.offset) == file_info.length {
        return Err(RustusError::FrozenFile);
    }
    let chunk_len = bytes.len();
    // Appending bytes to file.
    state.data_storage.add_bytes(&file_info, bytes).await?;
    // Updating offset.
    file_info.offset += chunk_len;
    // Saving info to info storage.
    state.info_storage.set_info(&file_info, false).await?;

    let mut hook = Hook::PostReceive;

    if file_info.length == Some(file_info.offset) {
        hook = Hook::PostFinish;
    }
    if state.config.hook_is_active(hook) {
        let message = state
            .config
            .notification_opts
            .hooks_format
            .format(&request, &file_info);
        let headers = request.headers().clone();
        tokio::task::spawn_local(async move {
            state
                .notification_manager
                .send_message(message, hook, &headers)
                .await
        });
    }

    #[cfg(feature = "metrics")]
    if hook == Hook::PostFinish {
        active_uploads.dec();
    }

    Ok(HttpResponse::NoContent()
        .insert_header(("Upload-Offset", file_info.offset.to_string()))
        .insert_header(CacheControl(vec![CacheDirective::NoCache]))
        .finish())
}

#[cfg(test)]
mod tests {
    use crate::{rustus_service, State};
    use actix_web::{
        http::StatusCode,
        test::{call_service, init_service, TestRequest},
        App,
    };

    #[actix_rt::test]
    /// Success test for writing bytes.
    ///
    /// This test creates file and writes bytes to it.
    async fn success() {
        let state = State::test_new().await;
        let mut rustus = init_service(App::new().configure(rustus_service(state.clone()))).await;
        let mut file = state.create_test_file().await;
        file.length = Some(100);
        file.offset = 0;
        state.info_storage.set_info(&file, false).await.unwrap();
        let test_data = "memes";
        let request = TestRequest::patch()
            .uri(state.config.file_url(file.id.as_str()).as_str())
            .insert_header(("Content-Type", "application/offset+octet-stream"))
            .insert_header(("Upload-Checksum", "md5 xIwpFX4rNYzBRAJ/Pi2MtA=="))
            .insert_header(("Upload-Offset", file.offset))
            .set_payload(test_data)
            .to_request();
        let resp = call_service(&mut rustus, request).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            resp.headers()
                .get("Upload-Offset")
                .unwrap()
                .to_str()
                .unwrap(),
            test_data.len().to_string().as_str()
        );
        let new_info = state
            .info_storage
            .get_info(file.id.clone().as_str())
            .await
            .unwrap();
        assert_eq!(new_info.offset, test_data.len());
    }

    #[actix_rt::test]
    /// Testing defer-length extension.
    ///
    /// During this test we'll try to update
    /// file's length while writing bytes to it.
    async fn success_update_file_length() {
        let state = State::test_new().await;
        let mut rustus = init_service(App::new().configure(rustus_service(state.clone()))).await;
        let mut file = state.create_test_file().await;
        file.length = None;
        file.deferred_size = true;
        file.offset = 0;
        state.info_storage.set_info(&file, false).await.unwrap();
        let test_data = "memes";
        let request = TestRequest::patch()
            .uri(state.config.file_url(file.id.as_str()).as_str())
            .param("file_id", file.id.clone())
            .insert_header(("Content-Type", "application/offset+octet-stream"))
            .insert_header(("Upload-Offset", file.offset))
            .insert_header(("Upload-Length", "20"))
            .set_payload(test_data)
            .to_request();
        let resp = call_service(&mut rustus, request).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            resp.headers()
                .get("Upload-Offset")
                .unwrap()
                .to_str()
                .unwrap(),
            test_data.len().to_string().as_str()
        );
        let new_info = state
            .info_storage
            .get_info(file.id.clone().as_str())
            .await
            .unwrap();
        assert_eq!(new_info.offset, test_data.len());
        assert_eq!(new_info.deferred_size, false);
        assert_eq!(new_info.length, Some(20));
    }

    #[actix_rt::test]
    /// Tests that if new file length
    /// is less than current offset, error is thrown.
    async fn new_file_length_lt_offset() {
        let state = State::test_new().await;
        let mut rustus = init_service(App::new().configure(rustus_service(state.clone()))).await;
        let mut file = state.create_test_file().await;
        file.length = None;
        file.deferred_size = true;
        file.offset = 30;
        state.info_storage.set_info(&file, false).await.unwrap();
        let test_data = "memes";
        let request = TestRequest::patch()
            .uri(state.config.file_url(file.id.as_str()).as_str())
            .insert_header(("Content-Type", "application/offset+octet-stream"))
            .insert_header(("Upload-Offset", file.offset))
            .insert_header(("Upload-Length", "20"))
            .set_payload(test_data)
            .to_request();
        let resp = call_service(&mut rustus, request).await;
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[actix_rt::test]
    /// Tests if user tries to update
    /// file length with known length,
    /// error is thrown.
    async fn new_file_length_size_already_known() {
        let state = State::test_new().await;
        let mut rustus = init_service(App::new().configure(rustus_service(state.clone()))).await;
        let mut file = state.create_test_file().await;
        file.length = Some(100);
        file.deferred_size = false;
        file.offset = 0;
        state.info_storage.set_info(&file, false).await.unwrap();
        let test_data = "memes";
        let request = TestRequest::patch()
            .uri(state.config.file_url(file.id.as_str()).as_str())
            .insert_header(("Content-Type", "application/offset+octet-stream"))
            .insert_header(("Upload-Offset", file.offset))
            .insert_header(("Upload-Length", "120"))
            .set_payload(test_data)
            .to_request();
        let resp = call_service(&mut rustus, request).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    /// Checks that if Content-Type header missing,
    /// wrong status code is returned.
    async fn no_content_header() {
        let state = State::test_new().await;
        let mut rustus = init_service(App::new().configure(rustus_service(state.clone()))).await;
        let mut file = state.create_test_file().await;
        file.length = Some(100);
        file.offset = 0;
        state.info_storage.set_info(&file, false).await.unwrap();
        let request = TestRequest::patch()
            .uri(state.config.file_url(file.id.as_str()).as_str())
            .insert_header(("Upload-Offset", "0"))
            .set_payload("memes")
            .to_request();
        let resp = call_service(&mut rustus, request).await;
        assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[actix_rt::test]
    /// Tests that method will return error if no offset header specified.
    async fn no_offset_header() {
        let state = State::test_new().await;
        let mut rustus = init_service(App::new().configure(rustus_service(state.clone()))).await;
        let mut file = state.create_test_file().await;
        file.length = Some(100);
        file.offset = 0;
        state.info_storage.set_info(&file, false).await.unwrap();
        let request = TestRequest::patch()
            .uri(state.config.file_url(file.id.as_str()).as_str())
            .insert_header(("Content-Type", "application/offset+octet-stream"))
            .set_payload("memes")
            .to_request();
        let resp = call_service(&mut rustus, request).await;
        assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[actix_rt::test]
    /// Tests that method will return error if wrong offset is passed.
    async fn wrong_offset_header() {
        let state = State::test_new().await;
        let mut rustus = init_service(App::new().configure(rustus_service(state.clone()))).await;
        let mut file = state.create_test_file().await;
        file.length = Some(100);
        file.offset = 0;
        state.info_storage.set_info(&file, false).await.unwrap();
        let request = TestRequest::patch()
            .uri(state.config.file_url(file.id.as_str()).as_str())
            .insert_header(("Upload-Offset", "1"))
            .insert_header(("Content-Type", "application/offset+octet-stream"))
            .set_payload("memes")
            .to_request();
        let resp = call_service(&mut rustus, request).await;
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[actix_rt::test]
    /// Tests that method would return error if file was already uploaded.
    async fn final_upload() {
        let state = State::test_new().await;
        let mut rustus = init_service(App::new().configure(rustus_service(state.clone()))).await;
        let mut file = state.create_test_file().await;
        file.is_final = true;
        state.info_storage.set_info(&file, false).await.unwrap();
        let request = TestRequest::patch()
            .uri(state.config.file_url(file.id.as_str()).as_str())
            .insert_header(("Upload-Offset", file.offset))
            .insert_header(("Content-Type", "application/offset+octet-stream"))
            .set_payload("memes")
            .to_request();
        let resp = call_service(&mut rustus, request).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    /// Tests that method would return 404 if file was saved in other storage.
    async fn wrong_storage() {
        let state = State::test_new().await;
        let mut rustus = init_service(App::new().configure(rustus_service(state.clone()))).await;
        let mut file = state.create_test_file().await;
        file.storage = "unknown".into();
        state.info_storage.set_info(&file, false).await.unwrap();
        let request = TestRequest::patch()
            .uri(state.config.file_url(file.id.as_str()).as_str())
            .insert_header(("Upload-Offset", file.offset))
            .insert_header(("Content-Type", "application/offset+octet-stream"))
            .set_payload("memes")
            .to_request();
        let resp = call_service(&mut rustus, request).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    /// Tests that method won't allow you to update
    /// file if it's offset already equal to length.
    async fn frozen_file() {
        let state = State::test_new().await;
        let mut rustus = init_service(App::new().configure(rustus_service(state.clone()))).await;
        let mut file = state.create_test_file().await;
        file.offset = 10;
        file.length = Some(10);
        state.info_storage.set_info(&file, false).await.unwrap();
        let request = TestRequest::patch()
            .uri(state.config.file_url(file.id.as_str()).as_str())
            .insert_header(("Upload-Offset", file.offset))
            .insert_header(("Content-Type", "application/offset+octet-stream"))
            .set_payload("memes")
            .to_request();
        let resp = call_service(&mut rustus, request).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    /// Tests that method will return 404 if
    /// unknown file_id is passed.
    async fn unknown_file_id() {
        let state = State::test_new().await;
        let mut rustus = init_service(App::new().configure(rustus_service(state.clone()))).await;
        let request = TestRequest::patch()
            .uri(state.config.file_url("unknown").as_str())
            .insert_header(("Upload-Offset", "0"))
            .insert_header(("Content-Type", "application/offset+octet-stream"))
            .set_payload("memes")
            .to_request();
        let resp = call_service(&mut rustus, request).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    /// Tests checksum validation.
    async fn wrong_checksum() {
        let state = State::test_new().await;
        let mut rustus = init_service(App::new().configure(rustus_service(state.clone()))).await;
        let mut file = state.create_test_file().await;
        file.offset = 0;
        file.length = Some(10);
        state.info_storage.set_info(&file, false).await.unwrap();
        let request = TestRequest::patch()
            .uri(state.config.file_url(file.id.as_str()).as_str())
            .insert_header(("Upload-Offset", "0"))
            .insert_header(("Upload-Checksum", "md5 K9opmNmw7hl9oUKgRH9nJQ=="))
            .insert_header(("Content-Type", "application/offset+octet-stream"))
            .set_payload("memes")
            .to_request();
        let resp = call_service(&mut rustus, request).await;
        assert_eq!(resp.status(), StatusCode::EXPECTATION_FAILED);
    }
}
