use crate::{config::Config, state::RustusState};

mod routes;

async fn logger(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<impl axum::response::IntoResponse, (axum::http::StatusCode, String)> {
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
    log::info!("{method} {uri} {status} {elapsed}");
    Ok(res)
}

async fn fallback() -> impl axum::response::IntoResponse {
    (axum::http::StatusCode::NOT_FOUND, "Not found")
}

pub async fn start_server(config: Config) -> anyhow::Result<()> {
    let state = RustusState::from_config(&config).await?;
    let app = axum::Router::new()
        .route("/", axum::routing::post(routes::create::create_route))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind((config.host.clone(), config.port)).await?;
    println!("Starting server at http://{}:{}", config.host, config.port);
    axum::serve(
        listener,
        axum::Router::new()
            .nest(&config.url, app)
            .fallback(fallback)
            .layer(axum::middleware::from_fn(logger)),
    )
    .await?;
    Ok(())
}
