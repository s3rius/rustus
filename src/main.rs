use std::str::FromStr;

use actix_web::http::Method;
use actix_web::{
    dev::{Server, Service},
    middleware, web, App, HttpServer,
};
use log::error;

use config::TuserConf;

use crate::storages::Storage;

mod config;
mod errors;
mod protocol;
mod routes;
mod storages;

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
pub fn create_server<S: Storage + 'static + Send>(
    storage: S,
    app_conf: TuserConf,
) -> Result<Server, std::io::Error> {
    let host = app_conf.host.clone();
    let port = app_conf.port;
    let workers = app_conf.workers;
    let mut server = HttpServer::new(move || {
        App::new()
            .data(app_conf.clone())
            .data(storage.clone())
            // Adds all routes.
            .configure(protocol::setup::<S>(app_conf.clone()))
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
    Ok(server.run())
}

/// Main program entrypoint.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_conf = TuserConf::from_args();
    simple_logging::log_to_stderr(app_conf.log_level);

    let storage = app_conf.storage.get_storage(&app_conf);
    if let Err(err) = storage.prepare().await {
        error!("{}", err);
        return Err(err.into());
    }
    let server = create_server(storage, app_conf)?;
    server.await
}
