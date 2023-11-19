use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};

use crate::{
    config::Config, errors::RustusResult, state::RustusState, utils::headers::HeaderMapExt,
};
use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, State},
    http::HeaderValue,
    Router, ServiceExt,
};
use tower::Layer;

mod cors;
mod routes;

async fn logger(
    State(config): State<Arc<Config>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    let default_addr = ConnectInfo(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(0, 0, 0, 0),
        8000,
    )));
    let socket = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .unwrap_or(&default_addr);
    let remote = req.headers().get_remote_ip(socket, config.behind_proxy);
    let method = req.method().to_string();
    let uri = req
        .uri()
        .path_and_query()
        .map(ToString::to_string)
        .unwrap_or_default();

    let time = std::time::Instant::now();
    let version = req.version();
    let response = next.run(req).await;
    #[allow(clippy::cast_precision_loss)]
    let elapsed = (time.elapsed().as_micros() as f64) / 1000.0;
    let status = response.status().as_u16();

    // log::log!(log::Level::Info, "ememe");
    if uri != "/health" {
        let mut level = log::Level::Info;
        if !response.status().is_success() {
            level = log::Level::Error;
        }
        log::log!(
            level,
            "\"{method} {uri} {version:?}\" \"-\" \"{status}\" \"{remote}\" \"{elapsed}\""
        );
    }

    response
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

    resp
}

async fn healthcheck() -> impl axum::response::IntoResponse {
    axum::http::StatusCode::OK
}

async fn fallback() -> impl axum::response::IntoResponse {
    (axum::http::StatusCode::NOT_FOUND, "Not found")
}

pub fn get_router(state: Arc<RustusState>) -> Router {
    let config = state.config.clone();
    axum::Router::new()
        .route("/", axum::routing::post(routes::create::handler))
        .route("/:upload_id", axum::routing::patch(routes::upload::handler))
        .route("/:upload_id", axum::routing::get(routes::get_file::handler))
        .route(
            "/:upload_id",
            axum::routing::delete(routes::delete::handler),
        )
        .route(
            "/:upload_id",
            axum::routing::head(routes::file_info::handler),
        )
        .route_layer(cors::layer(
            config.cors.clone(),
            &config.notification_config.hooks_http_proxy_headers,
        ))
        .route("/", axum::routing::options(routes::info::handler))
        .with_state(state)
        .route_layer(axum::middleware::from_fn_with_state(
            config.clone(),
            add_tus_header,
        ))
        .route_layer(DefaultBodyLimit::max(config.max_body_size))
}

/// Start the server.
/// Here we just create a state and router with all routes and middlewares.
///
/// Then we start accepting incoming requests.
///
/// # Errors
///
/// This function returns an error if the server fails to start.
pub async fn start(config: Config) -> RustusResult<()> {
    let listener = tokio::net::TcpListener::bind((config.host.clone(), config.port)).await?;
    log::info!("Starting server at http://{}:{}", config.host, config.port);
    let state = Arc::new(RustusState::from_config(&config).await?);

    let tus_app = get_router(state);
    let main_router = axum::Router::new()
        .route("/health", axum::routing::get(healthcheck))
        .nest(&config.url, tus_app)
        .fallback(fallback);

    let service = axum::middleware::from_fn(method_replacer).layer(
        axum::middleware::from_fn_with_state(Arc::new(config.clone()), logger).layer(main_router),
    );

    axum::serve(
        listener,
        service.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
