use actix_web::{guard, web};
mod routes;

/// Add creation extensions.
///
/// This extension allows you
/// to create file before sending data.

pub fn add_extension(web_app: &mut web::ServiceConfig) {
    web_app.service(
        // Post /base
        // URL for creating files.
        web::resource("/")
            .name("creation:create_file")
            .guard(guard::Post())
            .to(routes::create_file),
    );
}
