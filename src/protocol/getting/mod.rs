use actix_web::{guard, web};

mod routes;

/// Add getting extension.
///
/// This extension allows you
/// to get uploaded file.
///
/// This is unofficial extension.
#[cfg_attr(coverage, no_coverage)]
pub fn add_extension(web_app: &mut web::ServiceConfig) {
    web_app.service(
        // GET /base/file
        web::resource("{file_id}")
            .name("getting:get")
            .guard(guard::Get())
            .to(routes::get_file),
    );
}
