use actix_web::{guard, web};

use crate::RustusConf;

mod routes;

/// Add getting extension.
///
/// This extension allows you
/// to get uploaded file.
///
/// This is unofficial extension.
pub fn add_extension(web_app: &mut web::ServiceConfig, app_conf: &RustusConf) {
    web_app.service(
        // GET /base/file
        web::resource(app_conf.file_url().as_str())
            .name("getting:get")
            .guard(guard::Get())
            .to(routes::get_file),
    );
}
