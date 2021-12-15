use std::str::FromStr;

use actix_web::{
    App,
    dev::{Server, Service}, guard, HttpServer, middleware, web,
};
use actix_web::http::Method;
use log::error;

use config::TuserConf;

use crate::storages::Storage;

mod config;
mod errors;
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
pub fn create_server<T: Storage + 'static + Send>(
    storage: T,
    app_conf: TuserConf,
) -> Result<Server, std::io::Error> {
    let host = app_conf.host.clone();
    let port = app_conf.port;
    let workers = app_conf.workers;
    let base_url = format!(
        "/{}",
        app_conf
            .url
            .strip_prefix('/')
            .unwrap_or_else(|| app_conf.url.as_str())
    );
    let file_url = format!(
        "{}/{{file_id}}",
        base_url
            .strip_suffix('/')
            .unwrap_or_else(|| base_url.as_str())
    );
    let mut server = HttpServer::new(move || {
        App::new()
            .data(app_conf.clone())
            .data(storage.clone())
            .service(
                // PATCH /base/{file_id}
                // Main URL for uploading files.
                web::resource(base_url.as_str())
                    .name("server_info")
                    .guard(guard::Options())
                    .to(routes::server_info),
            )

            .service(
                // PATCH /base/{file_id}
                // Main URL for uploading files.
                web::resource(file_url.as_str())
                    .name("write_bytes")
                    .guard(guard::Patch())
                    .to(routes::write_bytes::<T>),
            )
            .service(
                // HEAD /base/{file_id}
                // Main URL for getting info about files.
                web::resource(file_url.as_str())
                    .name("file_info")
                    .guard(guard::Head())
                    // Header to prevent the client and/or proxies from caching the response.
                    .wrap(middleware::DefaultHeaders::new().header("Cache-Control", "no-store"))
                    .to(routes::get_file_info::<T>),
            )
            .service(
                // Post /base/{file_id}
                // URL for creating files.
                web::resource(base_url.as_str())
                    .name("create_file")
                    .guard(guard::Post())
                    .to(routes::create_file::<T>),
            )
            // Main middleware that appends TUS headers.
            .wrap(
                middleware::DefaultHeaders::new()
                    .header("Tus-Resumable", "1.0.0")
                    .header("Tus-Version", "1.0.0")
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
    let args = TuserConf::from_args();
    simple_logging::log_to_stderr(args.log_level);

    let storage_conf = args.clone();
    let storage = args.storage.get_storage(storage_conf);
    if let Err(err) = storage.prepare().await {
        error!("{}", err);
        return Err(err.into());
    }
    let server = create_server(storage, args)?;
    server.await
}
