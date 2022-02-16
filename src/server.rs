use crate::{protocol, State};
use actix_web::web::PayloadConfig;
use actix_web::{middleware, web};

pub fn rustus_service(state: web::Data<State>) -> Box<dyn Fn(&mut web::ServiceConfig)> {
    Box::new(move |web_app| {
        web_app.service(
            web::scope(state.config.base_url().as_str())
                .app_data(state.clone())
                .app_data(PayloadConfig::new(state.config.max_body_size))
                // Main middleware that appends TUS headers.
                .wrap(
                    middleware::DefaultHeaders::new()
                        .add(("Tus-Resumable", "1.0.0"))
                        .add(("Tus-Version", "1.0.0")),
                )
                .configure(protocol::setup(state.config.clone())),
        );
    })
}
