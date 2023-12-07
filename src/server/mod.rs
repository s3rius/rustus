use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};

use crate::{
    config::Config, errors::RustusResult, state::RustusState, utils::headers::HeaderMapExt,
};
use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, MatchedPath, Request, State},
    http::HeaderValue,
    response::Response,
    Router, ServiceExt,
};
use tower::Layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cors;
mod routes;

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
    let behind_proxy = config.behind_proxy;
    let default_addr = ConnectInfo(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(0, 0, 0, 0),
        0,
    )));

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            config.log_level,
        ))
        .with(
            tracing_subscriber::fmt::layer()
                .with_level(true)
                .with_file(false)
                .with_line_number(false)
                .with_target(false),
        )
        .with(sentry_tracing::layer())
        .init();

    let tracer = tower_http::trace::TraceLayer::new_for_http()
        .make_span_with(move |request: &Request| {
            let matched_path = request
                .extensions()
                .get::<MatchedPath>()
                .map(MatchedPath::as_str);
            let socket_addr = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .unwrap_or(&default_addr);
            let ip = request.headers().get_remote_ip(socket_addr, behind_proxy);
            tracing::info_span!(
                "request",
                method = ?request.method(),
                matched_path,
                version = ?request.version(),
                ip = ip,
                status = tracing::field::Empty,
            )
        })
        .on_response(
            |response: &Response, latency: Duration, span: &tracing::Span| {
                span.record("status", &response.status().as_u16());
                span.record("duration", latency.as_millis());
                tracing::info!("response");
            },
        );

    let state = Arc::new(RustusState::from_config(&config).await?);
    let tus_app = get_router(state);
    let mut main_router = axum::Router::new()
        .route("/health", axum::routing::get(healthcheck))
        .nest(&config.url, tus_app)
        .fallback(fallback)
        .layer(tracer);

    if config.sentry_config.dsn.is_some() {
        main_router = main_router
            .layer(sentry_tower::NewSentryLayer::new_from_top())
            .layer(sentry_tower::SentryHttpLayer::new());
    }

    let listener = tokio::net::TcpListener::bind((config.host.clone(), config.port)).await?;
    tracing::info!("Starting server at http://{}:{}", config.host, config.port);
    axum::serve(
        listener,
        axum::middleware::from_fn(method_replacer)
            .layer(main_router)
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
