use actix_web::{guard, web};

use crate::{Storage, TuserConf};

mod routes;

/// Add termination extension.
///
/// This extension allows you
/// to terminate file upload.
pub fn add_extension<S: Storage + 'static + Send>(
    web_app: &mut web::ServiceConfig,
    app_conf: &TuserConf,
) {
    web_app.service(
        // DELETE /base/file
        web::resource(app_conf.file_url().as_str())
            .name("termination:terminate")
            .guard(guard::Delete())
            .to(routes::terminate::<S>),
    );
}
