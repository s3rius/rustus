use actix_web::{guard, middleware, web};

use crate::RustusConf;

mod routes;

/// Add core TUS protocol endpoints.
///
/// This part of a protocol
/// has several endpoints.
///
/// OPTIONS /api    - to get info about the app.
/// HEAD /api/file  - to get info about the file.
/// PATCH /api/file - to add bytes to file.
pub fn add_extension(web_app: &mut web::ServiceConfig, app_conf: &RustusConf) {
    web_app
        .service(
            // PATCH /base/{file_id}
            // Main URL for uploading files.
            web::resource(app_conf.base_url().as_str())
                .name("core:server_info")
                .guard(guard::Options())
                .to(routes::server_info),
        )
        .service(
            // PATCH /base/{file_id}
            // Main URL for uploading files.
            web::resource(app_conf.file_url().as_str())
                .name("core:write_bytes")
                .guard(guard::Patch())
                .to(routes::write_bytes),
        )
        .service(
            // HEAD /base/{file_id}
            // Main URL for getting info about files.
            web::resource(app_conf.file_url().as_str())
                .name("core:file_info")
                .guard(guard::Head())
                // Header to prevent the client and/or proxies from caching the response.
                .wrap(middleware::DefaultHeaders::new().header("Cache-Control", "no-store"))
                .to(routes::get_file_info),
        );
}
