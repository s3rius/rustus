use crate::protocol::extensions::Extensions;
use actix_web::{http::StatusCode, web, HttpResponse, HttpResponseBuilder};

use crate::State;

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::unused_async)]
pub async fn server_info(state: web::Data<State>) -> HttpResponse {
    let ext_str = state
        .config
        .tus_extensions
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join(",");
    let mut response_builder = HttpResponseBuilder::new(StatusCode::OK);
    response_builder.insert_header(("Tus-Extension", ext_str.as_str()));
    if state.config.tus_extensions.contains(&Extensions::Checksum) {
        response_builder.insert_header(("Tus-Checksum-Algorithm", "md5,sha1,sha256,sha512"));
    }
    response_builder.finish()
}

#[cfg(test)]
mod tests {
    use crate::{protocol::extensions::Extensions, server::test::get_service, State};
    use actix_web::test::{call_service, TestRequest};

    use actix_web::http::Method;

    #[actix_rt::test]
    async fn test_server_info() {
        let mut state = State::test_new().await;
        let mut rustus = get_service(state.clone()).await;
        state.config.tus_extensions = vec![
            Extensions::Creation,
            Extensions::Concatenation,
            Extensions::Termination,
        ];
        let request = TestRequest::with_uri(state.config.test_url().as_str())
            .method(Method::OPTIONS)
            .to_request();
        let response = call_service(&mut rustus, request).await;
        let extensions = response
            .headers()
            .get("Tus-Extension")
            .unwrap()
            .to_str()
            .unwrap()
            .clone();
        assert!(extensions.contains(Extensions::Creation.to_string().as_str()));
        assert!(extensions.contains(Extensions::Concatenation.to_string().as_str()));
        assert!(extensions.contains(Extensions::Termination.to_string().as_str()));
    }
}
