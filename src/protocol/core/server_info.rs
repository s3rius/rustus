use actix_web::{web, HttpResponse};

use crate::State;

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::unused_async)]
pub async fn server_info(state: web::Data<State>) -> HttpResponse {
    let ext_str = state
        .config
        .extensions_vec()
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(",");
    HttpResponse::Ok()
        .insert_header(("Tus-Extension", ext_str.as_str()))
        .finish()
}

#[cfg(test)]
mod tests {
    use crate::protocol::extensions::Extensions;
    use crate::{rustus_service, State};
    use actix_web::test::{call_service, init_service, TestRequest};

    use actix_web::http::Method;
    use actix_web::{web, App};

    #[actix_rt::test]
    async fn test_server_info() {
        let mut state = State::test_new().await;
        let mut rustus = init_service(
            App::new().configure(rustus_service(web::Data::new(state.test_clone().await))),
        )
        .await;
        state.config.tus_extensions = vec![
            Extensions::Creation,
            Extensions::Concatenation,
            Extensions::Termination,
        ];
        let request = TestRequest::with_uri(state.config.base_url().as_str())
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
