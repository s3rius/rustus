use std::str::FromStr;
use std::sync::Arc;

use actix_web::http::Method;
use actix_web::{
    dev::{Server, Service},
    middleware, web, App, HttpServer,
};
use fern::colors::{Color, ColoredLevelConfig};
use fern::Dispatch;
use log::LevelFilter;

use crate::errors::RustusResult;
use crate::notifiers::models::notification_manager::NotificationManager;
use config::RustusConf;

use crate::storages::Storage;

mod config;
mod errors;
mod info_storages;
mod notifiers;
mod protocol;
mod routes;
mod storages;
mod utils;

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
    eprintln!("Base URL: {}", app_conf.base_url());
    eprintln!("Available extensions: {}", extensions);
    eprintln!("Enabled hooks: {}", hooks);
    eprintln!();
    eprintln!();
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
pub fn create_server(
    storage: Box<dyn Storage + Send + Sync>,
    app_conf: RustusConf,
    notification_manager: NotificationManager,
) -> Result<Server, std::io::Error> {
    let host = app_conf.host.clone();
    let port = app_conf.port;
    let workers = app_conf.workers;
    let app_conf_data = web::Data::new(app_conf.clone());
    let storage_data: web::Data<Box<dyn Storage + Send + Sync>> =
        web::Data::from(Arc::new(storage));
    let manager_data: web::Data<Box<NotificationManager>> =
        web::Data::from(Arc::new(Box::new(notification_manager)));
    let mut server = HttpServer::new(move || {
        App::new()
            .app_data(app_conf_data.clone())
            .app_data(storage_data.clone())
            .app_data(manager_data.clone())
            // Adds all routes.
            .configure(protocol::setup(app_conf.clone()))
            // Main middleware that appends TUS headers.
            .wrap(
                middleware::DefaultHeaders::new()
                    .add(("Tus-Resumable", "1.0.0"))
                    .add(("Tus-Max-Size", app_conf.max_body_size.to_string()))
                    .add(("Tus-Version", "1.0.0")),
            )
            .wrap(middleware::Logger::new("\"%r\" \"-\" \"%s\" \"%a\" \"%D\""))
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
            // Default response for unknown requests.
            // It returns 404 status_code.
            .default_service(web::route().to(routes::not_found))
    })
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
#[actix_web::main]
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
    let mut storage = app_conf.storage_opts.storage.get(&app_conf, info_storage);
    // Preparing it.
    storage.prepare().await?;

    // Creating notification manager.
    let notification_manager = NotificationManager::new(&app_conf).await?;

    // Creating actual server and running it.
    let server = create_server(storage, app_conf, notification_manager)?;
    server.await
}
