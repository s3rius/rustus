use actix_web::{guard, web};

use crate::{Storage, TuserConf};

mod routes;

pub fn add_extension<S: Storage + 'static + Send>(
    web_app: &mut web::ServiceConfig,
    app_conf: &TuserConf,
) {
    web_app.service(
        // Post /base
        // URL for creating files.
        web::resource(app_conf.base_url().as_str())
            .name("creation-with-upload:create_file")
            .guard(guard::Post())
            .to(routes::create_file::<S>),
    );
}
