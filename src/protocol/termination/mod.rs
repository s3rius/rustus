use actix_web::{guard, web};

use crate::RustusConf;

mod routes;

/// Add termination extension.
///
/// This extension allows you
/// to terminate file upload.
pub fn add_extension(web_app: &mut web::ServiceConfig, app_conf: &RustusConf) {
    web_app.service(
        // DELETE /base/file
        web::resource(app_conf.file_url().as_str())
            .name("termination:terminate")
            .guard(guard::Delete())
            .to(routes::terminate),
    );
}
