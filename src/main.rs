#![cfg_attr(coverage, feature(no_coverage))]

use std::{collections::HashMap, str::FromStr};

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
use log::{error, LevelFilter};

use config::RustusConf;

use metrics::{ActiveUploads, FinishedUploads, StartedUploads, TerminatedUploads, UploadSizes};
use wildmatch::WildMatch;

use crate::{
    errors::{RustusError, RustusResult},
    info_storages::InfoStorage,
    notifiers::models::notification_manager::NotificationManager,
    server::rustus_service,
    state::State,
    storages::Storage,
};

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

mod config;
mod errors;
mod info_storages;
mod metrics;
mod notifiers;
mod protocol;
mod routes;
mod server;
mod state;
mod storages;
mod utils;

#[cfg_attr(coverage, no_coverage)]
fn greeting(app_conf: &RustusConf) {
    let extensions = app_conf
        .extensions_vec()
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let hooks = app_conf
        .notification_opts
        .hooks
        .clone()
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let rustus_logo = include_str!("../imgs/rustus_startup_logo.txt");
    eprintln!("\n\n{}", rustus_logo);
    eprintln!("Welcome to rustus!");
    eprintln!("Base URL: /{}", app_conf.base_url());
    eprintln!("Available extensions: {}", extensions);
    eprintln!("Enabled hooks: {}", hooks);
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
        .allowed_headers(additional_headers.into_iter())
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
            WildMatch::new(origin.clone().as_str()) == request_origin.to_str().unwrap_or_default()
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
#[cfg_attr(coverage, no_coverage)]
#[allow(clippy::too_many_lines)]
pub fn create_server(state: State) -> RustusResult<Server> {
    let host = state.config.host.clone();
    let port = state.config.port;
    let cors_hosts = state.config.cors.clone();
    let workers = state.config.workers;
    #[cfg(feature = "http_notifier")]
    let proxy_headers = state
        .config
        .notification_opts
        .hooks_http_proxy_headers
        .clone();
    #[cfg(not(feature = "http_notifier"))]
    let proxy_headers = vec![];
    let metrics = actix_web_prom::PrometheusMetricsBuilder::new("")
        .endpoint("/metrics")
        .build()
        .map_err(|err| {
            error!("{}", err);
            RustusError::Unknown
        })?;
    let active_uploads = ActiveUploads::new()?;
    let upload_sizes = UploadSizes::new()?;
    let started_uploads = StartedUploads::new()?;
    let finished_uploads = FinishedUploads::new()?;
    let terminated_uploads = TerminatedUploads::new()?;
    let found_errors = prometheus::IntCounterVec::new(
        prometheus::Opts {
            namespace: "".into(),
            subsystem: "".into(),
            name: "errors".into(),
            help: "Found errors".into(),
            const_labels: HashMap::new(),
            variable_labels: Vec::new(),
        },
        &["path", "description"],
    )?;
    metrics
        .registry
        .register(Box::new(active_uploads.gauge.clone()))?;
    metrics
        .registry
        .register(Box::new(upload_sizes.hist.clone()))?;
    metrics
        .registry
        .register(Box::new(started_uploads.counter.clone()))?;
    metrics
        .registry
        .register(Box::new(finished_uploads.counter.clone()))?;
    metrics
        .registry
        .register(Box::new(terminated_uploads.counter.clone()))?;
    metrics.registry.register(Box::new(found_errors.clone()))?;
    let mut server = HttpServer::new(move || {
        let error_metrics = found_errors.clone();
        App::new()
            .app_data(web::Data::new(active_uploads.clone()))
            .app_data(web::Data::new(started_uploads.clone()))
            .app_data(web::Data::new(finished_uploads.clone()))
            .app_data(web::Data::new(terminated_uploads.clone()))
            .app_data(web::Data::new(upload_sizes.clone()))
            .route("/health", web::get().to(routes::health_check))
            .configure(rustus_service(state.clone()))
            .wrap(metrics.clone())
            .wrap(middleware::Logger::new("\"%r\" \"-\" \"%s\" \"%a\" \"%D\""))
            .wrap(create_cors(cors_hosts.clone(), proxy_headers.clone()))
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
                        let url = match srv_response.request().match_pattern() {
                            Some(pattern) => pattern,
                            None => "".into(),
                        };
                        let err_desc = format!("{}", err);
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

#[cfg_attr(coverage, no_coverage)]
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
        .level_for("rbatis", LevelFilter::Error)
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
#[cfg_attr(coverage, no_coverage)]
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app_conf = RustusConf::from_args();
    // Configuring logging.
    // I may change it to another log system like `fern` later, idk.
    setup_logging(&app_conf)?;
    // Printing cool message.
    greeting(&app_conf);

    // Creating info storage.
    // It's used to store info about files.
    let mut info_storage = app_conf
        .info_storage_opts
        .info_storage
        .get(&app_conf)
        .await?;
    // Preparing it, lol.
    info_storage.prepare().await?;

    // Creating file storage.
    let mut storage = app_conf.storage_opts.storage.get(&app_conf);
    // Preparing it.
    storage.prepare().await?;

    // Creating notification manager.
    let notification_manager = NotificationManager::new(&app_conf).await?;

    // Creating actual server and running it.
    let server = create_server(State::new(
        app_conf.clone(),
        storage,
        info_storage,
        notification_manager,
    ))?;
    server.await
}
