use actix_web::web;

use crate::config::ProtocolExtensions;
use crate::{Storage, TuserConf};

mod core;
mod creation;
mod creation_with_upload;
mod termination;

/// Configure TUS web application.
///
/// This function resolves all protocol extensions
/// provided by CLI into services and adds it to the application.
pub fn setup<S: Storage + 'static + Send>(
    app_conf: TuserConf,
) -> Box<dyn Fn(&mut web::ServiceConfig)> {
    Box::new(move |web_app| {
        for extension in app_conf.extensions_vec() {
            match extension {
                ProtocolExtensions::Creation => creation::add_extension::<S>(web_app, &app_conf),
                ProtocolExtensions::CreationWithUpload => {
                    creation_with_upload::add_extension::<S>(web_app, &app_conf);
                }
                ProtocolExtensions::Termination => {
                    termination::add_extension::<S>(web_app, &app_conf);
                }
            }
        }
        core::add_extension::<S>(web_app, &app_conf);
    })
}
