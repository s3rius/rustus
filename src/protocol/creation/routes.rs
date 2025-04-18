use actix_web::{web, web::Bytes, HttpRequest, HttpResponse};
use base64::{engine::general_purpose, Engine};
use std::collections::HashMap;

use crate::{
    data_storage::base::DataStorage,
    file_info::FileInfo,
    info_storage::base::InfoStorage,
    metrics,
    notifiers::Hook,
    protocol::extensions::Extensions,
    utils::headers::{check_header, parse_header},
    State,
};

/// Get metadata info from request.
///
/// Metadata is located in Upload-Metadata header.
/// Key and values are separated by spaces and
/// pairs are delimited with commas.
///
/// E.G.
/// `Upload-Metadata: Video bWVtZXM=,Category bWVtZXM=`
///
/// All values are encoded as base64 strings.
fn get_metadata(request: &HttpRequest) -> Option<HashMap<String, String>> {
    request
        .headers()
        .get("Upload-Metadata")
        .and_then(|her| her.to_str().ok())
        .map(String::from)
        .map(|header_string| {
            let mut meta_map = HashMap::new();
            for meta_pair in header_string.split(',') {
                let mut split = meta_pair.trim().split(' ');
                let key = split.next();
                let b64val = split.next();
                if key.is_none() || b64val.is_none() {
                    continue;
                }
                let value = general_purpose::STANDARD
                    .decode(b64val.unwrap())
                    .ok()
                    .and_then(|value| String::from_utf8(value).ok());
                if let Some(res) = value {
                    meta_map.insert(String::from(key.unwrap()), res);
                }
            }
            meta_map
        })
}

fn get_upload_parts(request: &HttpRequest) -> Vec<String> {
    let concat_header = request.headers().get("Upload-Concat").unwrap();
    let header_str = concat_header.to_str().unwrap();
    let urls = header_str.strip_prefix("final;").unwrap();

    urls.split(' ')
        .filter_map(|val: &str| val.trim().split('/').last().map(String::from))
        .filter(|val| val.trim() != "")
        .collect()
}

/// Create file.
///
/// This method allows you to create file to start uploading.
///
/// This method supports defer-length if
/// you don't know actual file length and
/// you can upload first bytes if creation-with-upload
/// extension is enabled.
#[allow(clippy::too_many_lines)]
pub async fn create_file(
    metrics: web::Data<metrics::RustusMetrics>,
    state: web::Data<State>,
    request: HttpRequest,
    bytes: Bytes,
) -> actix_web::Result<HttpResponse> {
    // Getting Upload-Length header value as usize.
    let length = parse_header(&request, "Upload-Length");

    // With this option enabled,
    // we have to check whether length is a non-zero number.
    if !state.config.allow_empty && length == Some(0) {
        return Ok(HttpResponse::BadRequest().body("Upload-Length should be greater than zero"));
    }

    // Checking Upload-Defer-Length header.
    let defer_size = check_header(&request, "Upload-Defer-Length", |val| val == "1");

    // Indicator that creation-defer-length is enabled.
    let defer_ext = state
        .config
        .tus_extensions
        .contains(&Extensions::CreationDeferLength);

    let is_final = check_header(&request, "Upload-Concat", |val| val.starts_with("final;"));

    let concat_ext = state
        .config
        .tus_extensions
        .contains(&Extensions::Concatenation);

    // Check that Upload-Length header is provided.
    // Otherwise checking that defer-size feature is enabled
    // and header provided.
    if length.is_none() && !((defer_ext && defer_size) || (concat_ext && is_final)) {
        return Ok(HttpResponse::BadRequest().body("Upload-Length header is required"));
    }

    if state.config.max_file_size.is_some() && state.config.max_file_size < length {
        return Ok(HttpResponse::BadRequest().body(format!(
            "Upload-Length should be less than or equal to {}",
            state.config.max_file_size.unwrap()
        )));
    }

    let meta = get_metadata(&request);

    let file_id = uuid::Uuid::new_v4().to_string();
    let mut file_info = FileInfo::new(
        file_id.as_str(),
        length,
        None,
        state.data_storage.get_name().to_string(),
        meta,
    );

    let is_partial = check_header(&request, "Upload-Concat", |val| val == "partial");

    if concat_ext {
        if is_final {
            file_info.is_final = true;
            let upload_parts = get_upload_parts(&request);
            if upload_parts.is_empty() {
                return Ok(HttpResponse::BadRequest()
                    .body("Upload-Concat header has no parts to create final upload."));
            }
            file_info.parts = Some(upload_parts);
            file_info.deferred_size = false;
        }
        if is_partial {
            file_info.is_partial = true;
        }
    }

    if state.config.hook_is_active(Hook::PreCreate) {
        let message = state.config.notification_opts.hooks_format.format(
            &request,
            &file_info,
            state.config.notification_opts.behind_proxy,
        );
        let headers = request.headers();
        let cloned_info = file_info.clone();
        state
            .notification_manager
            .send_message(message, Hook::PreCreate, &cloned_info, headers)
            .await?;
    }

    // Create file and get the it's path.
    file_info.path = Some(state.data_storage.create_file(&mut file_info).await?);

    // Incrementing number of active uploads

    metrics.active_uploads.inc();
    metrics.started_uploads.inc();

    if let Some(length) = file_info.length {
        #[allow(clippy::cast_precision_loss)]
        metrics.upload_sizes.observe(length as f64);
    }

    if file_info.is_final {
        let mut final_size = 0;
        let mut parts_info = Vec::new();
        for part_id in file_info.clone().parts.unwrap() {
            let part = state.info_storage.get_info(part_id.as_str()).await?;
            if part.length != Some(part.offset) {
                return Ok(
                    HttpResponse::BadRequest().body(format!("{} upload is not complete.", part.id))
                );
            }
            if !part.is_partial {
                return Ok(
                    HttpResponse::BadRequest().body(format!("{} upload is not partial.", part.id))
                );
            }
            final_size += &part.length.unwrap();
            parts_info.push(part.clone());
        }
        state
            .data_storage
            .concat_files(&file_info, parts_info.clone())
            .await?;
        file_info.offset = final_size;
        file_info.length = Some(final_size);
        if state.config.remove_parts {
            for part in parts_info {
                state.data_storage.remove_file(&part).await?;
                state.info_storage.remove_info(part.id.as_str()).await?;
            }
        }
    }

    // Checking if creation-with-upload extension is enabled.
    let with_upload = state
        .config
        .tus_extensions
        .contains(&Extensions::CreationWithUpload);
    if with_upload && !bytes.is_empty() && !(concat_ext && is_final) {
        let octet_stream = |val: &str| val == "application/offset+octet-stream";
        if check_header(&request, "Content-Type", octet_stream) {
            // Writing first bytes.
            let chunk_len = bytes.len();
            // Appending bytes to file.
            state.data_storage.add_bytes(&mut file_info, bytes).await?;
            // Updating offset.
            file_info.offset += chunk_len;
        }
    }

    state.info_storage.set_info(&file_info, true).await?;

    // It's more intuitive to send post-finish
    // hook, when final upload is created.
    // https://github.com/s3rius/rustus/issues/77
    let post_hook = if file_info.is_final || Some(file_info.offset) == file_info.length {
        Hook::PostFinish
    } else {
        Hook::PostCreate
    };

    if state.config.hook_is_active(post_hook) {
        let message = state.config.notification_opts.hooks_format.format(
            &request,
            &file_info,
            state.config.notification_opts.behind_proxy,
        );
        let headers = request.headers().clone();
        // Adding send_message task to tokio reactor.
        // Thin function would be executed in background.
        let cloned_info = file_info.clone();
        tokio::task::spawn_local(async move {
            state
                .notification_manager
                .send_message(message, post_hook, &cloned_info, &headers)
                .await
        });
    }

    // Create upload URL for this file.
    let upload_url = request.url_for("core:write_bytes", [file_info.id.clone()])?;

    Ok(HttpResponse::Created()
        .insert_header((
            "Location",
            upload_url
                .as_str()
                .strip_suffix('/')
                .unwrap_or(upload_url.as_str()),
        ))
        .insert_header(("Upload-Offset", file_info.offset.to_string()))
        .finish())
}

#[cfg(test)]
mod tests {
    use crate::{info_storage::base::InfoStorage, server::test::get_service, State};
    use actix_web::{
        http::StatusCode,
        test::{call_service, TestRequest},
        web,
    };
    use base64::{engine::general_purpose, Engine};

    #[actix_rt::test]
    async fn success() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .insert_header(("Upload-Length", 100))
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        // Getting file from location header.
        let item_id = resp
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap()
            .split('/')
            .last()
            .unwrap();
        let file_info = state.info_storage.get_info(item_id).await.unwrap();
        assert_eq!(file_info.length, Some(100));
        assert_eq!(file_info.offset, 0);
    }

    #[actix_rt::test]
    async fn wrong_length() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .insert_header(("Upload-Length", 0))
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    async fn allow_empty() {
        let mut state = State::test_new().await;
        state.config.allow_empty = true;
        let rustus = get_service(state.clone()).await;
        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .insert_header(("Upload-Length", 0))
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[actix_rt::test]
    async fn success_with_bytes() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let test_data = "memes";
        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .insert_header(("Upload-Length", 100))
            .insert_header(("Content-Type", "application/offset+octet-stream"))
            .set_payload(web::Bytes::from(test_data))
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        // Getting file from location header.
        let item_id = resp
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap()
            .split('/')
            .last()
            .unwrap();
        let file_info = state.info_storage.get_info(item_id).await.unwrap();
        assert_eq!(file_info.length, Some(100));
        assert_eq!(file_info.offset, test_data.len());
    }

    #[actix_rt::test]
    async fn with_bytes_wrong_content_type() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let test_data = "memes";
        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .insert_header(("Upload-Length", 100))
            .insert_header(("Content-Type", "random"))
            .set_payload(web::Bytes::from(test_data))
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        // Getting file from location header.
        let item_id = resp
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap()
            .split('/')
            .last()
            .unwrap();
        let file_info = state.info_storage.get_info(item_id).await.unwrap();
        assert_eq!(file_info.length, Some(100));
        assert_eq!(file_info.offset, 0);
    }

    #[actix_rt::test]
    async fn success_defer_size() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .insert_header(("Upload-Defer-Length", "1"))
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        // Getting file from location header.
        let item_id = resp
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap()
            .split('/')
            .last()
            .unwrap();
        let file_info = state.info_storage.get_info(item_id).await.unwrap();
        assert_eq!(file_info.length, None);
        assert!(file_info.deferred_size);
    }

    #[actix_rt::test]
    async fn success_partial_upload() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .insert_header(("Upload-Length", 100))
            .insert_header(("Upload-Concat", "partial"))
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        // Getting file from location header.
        let item_id = resp
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap()
            .split('/')
            .last()
            .unwrap();
        let file_info = state.info_storage.get_info(item_id).await.unwrap();
        assert_eq!(file_info.length, Some(100));
        assert!(file_info.is_partial);
        assert!(!file_info.is_final);
    }

    #[actix_rt::test]
    async fn success_final_upload() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let mut part1 = state.create_test_file().await;
        let mut part2 = state.create_test_file().await;
        part1.is_partial = true;
        part1.length = Some(100);
        part1.offset = 100;

        part2.is_partial = true;
        part2.length = Some(100);
        part2.offset = 100;

        state.info_storage.set_info(&part1, false).await.unwrap();
        state.info_storage.set_info(&part2, false).await.unwrap();

        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .insert_header(("Upload-Length", 100))
            .insert_header((
                "Upload-Concat",
                format!("final;/files/{} /files/{}", part1.id, part2.id),
            ))
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        // Getting file from location header.
        let item_id = resp
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap()
            .split('/')
            .last()
            .unwrap();
        let file_info = state.info_storage.get_info(item_id).await.unwrap();
        assert_eq!(file_info.length, Some(200));
        assert!(file_info.is_final);
    }

    #[actix_rt::test]
    async fn invalid_final_upload_no_parts() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;

        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .insert_header(("Upload-Length", 100))
            .insert_header(("Upload-Concat", "final;"))
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    async fn success_with_metadata() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .insert_header(("Upload-Length", 100))
            .insert_header((
                "Upload-Metadata",
                format!(
                    "test {}, pest {}",
                    general_purpose::STANDARD.encode("data1"),
                    general_purpose::STANDARD.encode("data2")
                ),
            ))
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        // Getting file from location header.
        let item_id = resp
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap()
            .split('/')
            .last()
            .unwrap();
        let file_info = state.info_storage.get_info(item_id).await.unwrap();
        assert_eq!(file_info.length, Some(100));
        assert_eq!(file_info.metadata.get("test").unwrap(), "data1");
        assert_eq!(file_info.metadata.get("pest").unwrap(), "data2");
        assert_eq!(file_info.offset, 0);
    }

    #[actix_rt::test]
    async fn success_with_metadata_wrong_encoding() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .insert_header(("Upload-Length", 100))
            .insert_header((
                "Upload-Metadata",
                format!(
                    "test data1, pest {}",
                    general_purpose::STANDARD.encode("data")
                ),
            ))
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        // Getting file from location header.
        let item_id = resp
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap()
            .split('/')
            .last()
            .unwrap();
        let file_info = state.info_storage.get_info(item_id).await.unwrap();
        assert_eq!(file_info.length, Some(100));
        assert!(!file_info.metadata.contains_key("test"));
        assert_eq!(file_info.metadata.get("pest").unwrap(), "data");
        assert_eq!(file_info.offset, 0);
    }

    #[actix_rt::test]
    async fn no_length_header() {
        let state = State::test_new().await;
        let rustus = get_service(state.clone()).await;
        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    async fn max_file_size_exceeded() {
        let mut state = State::test_new().await;
        state.config.max_file_size = Some(1000);
        let rustus = get_service(state.clone()).await;
        let request = TestRequest::post()
            .uri(state.config.test_url().as_str())
            .insert_header(("Upload-Length", 1001))
            .to_request();
        let resp = call_service(&rustus, request).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
