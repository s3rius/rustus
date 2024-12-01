use actix_web::{guard, web};

mod routes;

/// Add termination extension.
///
/// This extension allows you
/// to terminate file upload.

pub fn add_extension(web_app: &mut web::ServiceConfig) {
    web_app.service(
        // DELETE /base/file
        web::resource("/{file_id}/")
            .name("termination:terminate")
            .guard(guard::Delete())
            .to(routes::terminate),
    );
}
