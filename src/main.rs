use std::str::FromStr;
use std::sync::Arc;

use actix_web::http::Method;
use actix_web::{
    dev::{Server, Service},
    middleware, web, App, HttpServer,
};
use log::{error, info};

use config::RustusConf;

use crate::storages::Storage;

mod config;
mod errors;
mod info_storages;
mod protocol;
mod routes;
mod storages;
mod utils;

fn greeting(app_conf: &RustusConf) {
    let extensions = app_conf
        .extensions_vec()
        .into_iter()
        .map(String::from)
        .collect::<Vec<String>>()
        .join(",");
    info!("Welcome to rustus!");
    info!("Base URL: {}", app_conf.base_url());
    info!("Available extensions {}", extensions);
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
) -> Result<Server, std::io::Error> {
    let host = app_conf.host.clone();
    let port = app_conf.port;
    let workers = app_conf.workers;
    let storage_data: web::Data<Box<dyn Storage + Send + Sync>> =
        web::Data::from(Arc::new(storage));
    let mut server = HttpServer::new(move || {
        App::new()
            .data(app_conf.clone())
            .app_data(storage_data.clone())
            // Adds all routes.
            .configure(protocol::setup(app_conf.clone()))
            // Main middleware that appends TUS headers.
            .wrap(
                middleware::DefaultHeaders::new()
                    .header("Tus-Resumable", "1.0.0")
                    .header("Tus-Version", "1.0.0"),
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
    server = server.server_hostname("meme");
    Ok(server.run())
}

/// Main program entrypoint.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_conf = RustusConf::from_args();
    simple_logging::log_to_stderr(app_conf.log_level);

    let mut info_storage = app_conf
        .info_storage_opts
        .info_storage
        .get(&app_conf)
        .await?;
    info_storage.prepare().await?;
    let mut storage = app_conf.storage_opts.storage.get(&app_conf, info_storage);
    if let Err(err) = storage.prepare().await {
        error!("{}", err);
        return Err(err.into());
    }
    greeting(&app_conf);
    let server = create_server(storage, app_conf)?;
    server.await
}
