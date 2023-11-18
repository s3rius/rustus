use std::net::SocketAddr;

use crate::{
    config::Config, errors::RustusResult, server::cors::cors_layer, state::RustusState,
    utils::headers::HeaderMapExt,
};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::HeaderValue,
    Router, ServiceExt,
};
use tower::Layer;

mod cors;
mod routes;

async fn logger(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    let method = req.method().to_string();
    let uri = req
        .uri()
        .path_and_query()
        .map(ToString::to_string)
        .unwrap_or_default();

    let time = std::time::Instant::now();
    let res = next.run(req).await;
    let elapsed = time.elapsed().as_micros();
    let status = res.status().as_u16();

    // log::log!(log::Level::Info, "ememe");
    if uri != "/health" {
        let mut level = log::Level::Info;
        if res.status().is_server_error() {
            level = log::Level::Error;
            log::error!("{:#?}", res.body());
        }
        log::log!(level, "{method} {uri} {status} {elapsed}");
    }

    res
}

async fn method_replacer(
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    if let Some(new_method) = req.headers().get_method_override() {
        *req.method_mut() = new_method;
        req.headers_mut().remove("X-HTTP-Method-Override");
    }
    next.run(req).await
}

async fn add_tus_header(
    State(state): State<Config>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    let mut resp = next.run(req).await;
    resp.headers_mut()
        .insert("Tus-Resumable", HeaderValue::from_static("1.0.0"));
    resp.headers_mut()
        .insert("Tus-Version", HeaderValue::from_static("1.0.0"));

    let max_file_size = state
        .max_file_size
        .map(|val| val.to_string())
        .and_then(|val| HeaderValue::from_str(val.as_str()).ok());

    if let Some(max_size) = max_file_size {
        resp.headers_mut().insert("Tus-Max-Size", max_size);
    }

    return resp;
}

async fn healthcheck() -> impl axum::response::IntoResponse {
    axum::http::StatusCode::OK
}

async fn fallback() -> impl axum::response::IntoResponse {
    (axum::http::StatusCode::NOT_FOUND, "Not found")
}

pub fn get_router(state: RustusState) -> Router {
    let config = state.config.clone();
    axum::Router::new()
        .route("/", axum::routing::post(routes::create::create_upload))
        .route(
            "/:upload_id",
            axum::routing::patch(routes::upload::upload_chunk),
        )
        .route(
            "/:upload_id",
            axum::routing::get(routes::get_file::get_upload),
        )
        .route(
            "/:upload_id",
            axum::routing::delete(routes::delete::delete_upload),
        )
        .route(
            "/:upload_id",
            axum::routing::head(routes::file_info::get_file_info),
        )
        .route_layer(cors_layer(
            config.cors.clone(),
            config.notification_config.hooks_http_proxy_headers.clone(),
        ))
        .route("/", axum::routing::options(routes::info::get_server_info))
        .with_state(state)
        .route_layer(axum::middleware::from_fn_with_state(
            config.clone(),
            add_tus_header,
        ))
        .route_layer(DefaultBodyLimit::max(config.max_body_size))
}

pub async fn start_server(config: Config) -> RustusResult<()> {
    let listener = tokio::net::TcpListener::bind((config.host.clone(), config.port)).await?;
    log::info!("Starting server at http://{}:{}", config.host, config.port);
    let state = RustusState::from_config(&config).await?;

    let tus_app = get_router(state);
    let main_router = axum::Router::new()
        .route("/health", axum::routing::get(healthcheck))
        .nest(&config.url, tus_app)
        .fallback(fallback);

    let service = axum::middleware::from_fn(method_replacer)
        .layer(axum::middleware::from_fn(logger).layer(main_router));
    axum::serve(
        listener,
        service.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
