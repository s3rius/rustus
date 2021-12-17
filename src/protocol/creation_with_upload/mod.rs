use actix_web::{guard, web};

use crate::TuserConf;

mod routes;

pub fn add_extension(web_app: &mut web::ServiceConfig, app_conf: &TuserConf) {
    web_app.service(
        // Post /base
        // URL for creating files.
        web::resource(app_conf.base_url().as_str())
            .name("creation-with-upload:create_file")
            .guard(guard::Post())
            .to(routes::create_file),
    );
}
