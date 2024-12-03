#![warn(
    // Base lints.
    clippy::all,
    // Some pedantic lints.
    clippy::pedantic,
    // New lints which are cool.
    clippy::nursery,
)]
#![
    allow(
        // I don't care about this.
        clippy::module_name_repetitions, 
        // Yo, the hell you should put
        // it in docs, if signature is clear as sky.
        clippy::missing_errors_doc,
        // Actix is buit upon ?Send,
        // to maximize performance of a single thread.
        clippy::future_not_send
    )
]
use std::str::FromStr;

use actix_cors::Cors;
use actix_web::{
    dev::{Server, Service},
    http::{KeepAlive, Method},
    middleware, web, App, HttpServer,
};
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
use log::error;

use config::RustusConf;

use metrics::RustusMetrics;
use wildmatch::WildMatch;

use crate::{
    errors::{RustusError, RustusResult},
    server::rustus_service,
    state::State,
};

mod config;
mod data_storage;
mod errors;
mod file_info;
mod info_storage;
mod metrics;
mod notifiers;
mod protocol;
mod routes;
mod server;
mod state;
mod utils;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn greeting(app_conf: &RustusConf) {
    let extensions = app_conf
        .tus_extensions
        .clone()
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let hooks = app_conf
        .notification_opts
        .hooks
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join(", ");
    let rustus_logo = include_str!("../imgs/rustus_startup_logo.txt");
    eprintln!("\n\n{rustus_logo}");
    eprintln!("Welcome to rustus!");
    eprintln!("Base URL: /{}", app_conf.base_url());
    eprintln!("Available extensions: {extensions}");
    eprintln!("Enabled hooks: {hooks}");
    eprintln!();
    eprintln!();
}

/// Create CORS rules for the server.
///
/// CORS rules are applied to every handler.
///
/// If the origins vector is empty all origins are
/// welcome, otherwise it will create a wildcard match for
/// every host.
fn create_cors(origins: Vec<String>, additional_headers: Vec<String>) -> Cors {
    let mut cors = Cors::default()
        .allowed_methods(vec!["OPTIONS", "GET", "HEAD", "POST", "PATCH", "DELETE"])
        .allowed_headers(vec![
            "Content-Type",
            "Upload-Offset",
            "Upload-Checksum",
            "Upload-Length",
            "Upload-Metadata",
            "Upload-Concat",
            "Upload-Defer-Length",
            "Tus-Resumable",
            "Tus-Version",
            "X-HTTP-Method-Override",
            "Authorization",
            "Origin",
            "X-Requested-With",
            "X-Request-ID",
            "X-HTTP-Method-Override",
        ])
        .allowed_headers(additional_headers)
        .expose_headers(vec![
            "Location",
            "Tus-Version",
            "Tus-Resumable",
            "Tus-Max-Size",
            "Tus-Extension",
            "Tus-Checksum-Algorithm",
            "Content-Type",
            "Content-Length",
            "Upload-Length",
            "Upload-Metadata",
            "Upload-Defer-Length",
            "Upload-Concat",
            "Upload-Offset",
        ])
        .max_age(86400);

    // We allow any origin by default if no origin is specified.
    if origins.is_empty() {
        return cors.allow_any_origin();
    }

    // Adding origins.
    for origin in origins {
        cors = cors.allowed_origin_fn(move |request_origin, _| {
            WildMatch::new(&origin) == request_origin.to_str().unwrap_or_default()
        });
    }

    cors
}

/// Creates Actix server.
///
/// This function is parametrized with
/// Storage class.
///
/// This storage can later be used in
/// handlers.
///
/// # Errors
///
/// This function may throw an error
/// if the server can't be bound to the
/// given address.
#[allow(clippy::too_many_lines)]
pub fn create_server(state: State) -> RustusResult<Server> {
    let host = state.config.host.clone();
    let port = state.config.port;
    let disable_health_log = state.config.disable_health_access_log;
    let cors_hosts = state.config.cors.clone();
    let workers = state.config.workers;
    let proxy_headers = state
        .config
        .notification_opts
        .hooks_http_proxy_headers
        .clone();
    let metrics = RustusMetrics::new()?;
    let metrics_middleware = actix_web_prom::PrometheusMetricsBuilder::new("")
        .endpoint("/metrics")
        .registry(metrics.registry.clone())
        .build()
        .map_err(|err| {
            error!("{}", err);
            RustusError::Unknown
        })?;
    let mut server = HttpServer::new(move || {
        let mut logger = middleware::Logger::new("\"%r\" \"-\" \"%s\" \"%a\" \"%D\"");
        if disable_health_log {
            logger = logger.exclude("/health");
        }
        let error_metrics = metrics.found_errors.clone();
        App::new()
            .app_data(web::Data::new(metrics.clone()))
            .route("/health", web::get().to(routes::health_check))
            .configure(rustus_service(state.clone()))
            .wrap(metrics_middleware.clone())
            .wrap(logger)
            .wrap(create_cors(cors_hosts.clone(), proxy_headers.clone()))
            .wrap(sentry_actix::Sentry::new())
            // Middleware that overrides method of a request if
            // "X-HTTP-Method-Override" header is provided.
            .wrap_fn(|mut req, srv| {
                if let Some(header_value) = req.headers_mut().get("X-HTTP-Method-Override") {
                    if let Ok(method_name) = header_value.to_str() {
                        if let Ok(method) = Method::from_str(method_name) {
                            req.head_mut().method = method;
                        }
                    }
                }
                srv.call(req)
            })
            // This is middleware that registers found errors.
            .wrap_fn(move |req, srv| {
                // Call the service to resolve handler and return response.
                let fut = srv.call(req);
                // We need this copy, since we use it in moved closure later.
                let error_counter = error_metrics.clone();
                async move {
                    let srv_response = fut.await?;
                    if let Some(err) = srv_response.response().error() {
                        let url = srv_response.request().match_pattern().unwrap_or_default();
                        let err_desc = format!("{err}");
                        error_counter
                            .clone()
                            .with_label_values(&[url.as_str(), err_desc.as_str()])
                            .inc();
                    }
                    Ok(srv_response)
                }
            })
            // Default response for unknown requests.
            // It returns 404 status_code.
            .default_service(web::route().to(routes::not_found))
    })
    .keep_alive(KeepAlive::Disabled)
    .bind((host, port))?;

    // If custom workers count variable is provided.
    if let Some(workers_count) = workers {
        server = server.workers(workers_count);
    }

    Ok(server.run())
}

fn setup_logging(app_config: &RustusConf) -> RustusResult<()> {
    let colors = ColoredLevelConfig::new()
        // use builder methods
        .info(Color::Green)
        .warn(Color::Yellow)
        .debug(Color::BrightCyan)
        .error(Color::BrightRed)
        .trace(Color::Blue);

    Dispatch::new()
        .level(app_config.log_level)
        .chain(std::io::stdout())
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S%:z]"),
                colors.color(record.level()),
                message
            ));
        })
        .apply()?;
    Ok(())
}

/// Main program entrypoint.
#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    let app_conf = RustusConf::from_args();
    // Configuring logging.
    // I may change it to another log system like `fern` later, idk.
    setup_logging(&app_conf)?;

    #[allow(clippy::collection_is_never_read)]
    let mut _guard = None;
    if let Some(dsn) = &app_conf.sentry_opts.dsn {
        log::info!("Setting up sentry .");
        _guard = Some(sentry::init((
            dsn.as_str(),
            sentry::ClientOptions {
                debug: true,
                sample_rate: app_conf.sentry_opts.sample_rate,
                ..Default::default()
            },
        )));
    }

    // Printing cool message.
    greeting(&app_conf);

    let state = State::new(app_conf.clone()).await?;

    // Creating actual server and running it.
    let server = create_server(state)?;
    server.await
}
