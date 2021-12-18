use actix_web::web;

use crate::config::ProtocolExtensions;
use crate::RustusConf;

mod core;
mod creation;
mod getting;
mod termination;

/// Configure TUS web application.
///
/// This function resolves all protocol extensions
/// provided by CLI into services and adds it to the application.
pub fn setup(app_conf: RustusConf) -> Box<dyn Fn(&mut web::ServiceConfig)> {
    Box::new(move |web_app| {
        for extension in app_conf.extensions_vec() {
            match extension {
                ProtocolExtensions::Creation => creation::add_extension(web_app, &app_conf),
                ProtocolExtensions::Termination => {
                    termination::add_extension(web_app, &app_conf);
                }
                ProtocolExtensions::Getting => {
                    getting::add_extension(web_app, &app_conf);
                }
                _ => {}
            }
        }
        core::add_extension(web_app, &app_conf);
    })
}
