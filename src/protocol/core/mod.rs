use actix_web::{guard, middleware, web};

mod get_info;
mod server_info;
mod write_bytes;

/// Add core TUS protocol endpoints.
///
/// This part of a protocol
/// has several endpoints.
///
/// OPTIONS /api    - to get info about the app.
/// HEAD /api/file  - to get info about the file.
/// PATCH /api/file - to add bytes to file.
pub fn add_extension(web_app: &mut web::ServiceConfig) {
    web_app
        .service(
            // PATCH /base/{file_id}
            // Main URL for uploading files.
            web::resource("/")
                .name("core:server_info")
                .guard(guard::Options())
                .to(server_info::server_info),
        )
        .service(
            // PATCH /base/{file_id}
            // Main URL for uploading files.
            web::resource("/{file_id}/")
                .name("core:write_bytes")
                .guard(guard::Patch())
                .to(write_bytes::write_bytes),
        )
        .service(
            // HEAD /base/{file_id}
            // Main URL for getting info about files.
            web::resource("/{file_id}/")
                .name("core:file_info")
                .guard(guard::Head())
                // Header to prevent the client and/or proxies from caching the response.
                .wrap(middleware::DefaultHeaders::new().add(("Cache-Control", "no-store")))
                .to(get_info::get_file_info),
        );
}
