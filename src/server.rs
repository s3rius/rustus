use crate::{protocol, State};
use actix_web::{middleware, web, web::PayloadConfig};

pub fn rustus_service(state: State) -> Box<dyn Fn(&mut web::ServiceConfig)> {
    Box::new(move |web_app| {
        web_app.service(
            web::scope(state.config.base_url().as_str())
                .app_data(web::Data::new(state.clone()))
                .app_data(PayloadConfig::new(state.config.max_body_size))
                .wrap(middleware::NormalizePath::new(
                    middleware::TrailingSlash::Always,
                ))
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

#[cfg(test)]
pub mod test {
    use super::rustus_service;
    use crate::{metrics, state::State};
    use actix_web::{dev::ServiceResponse, test::init_service, web, App};

    pub async fn get_service(
        state: State,
    ) -> impl actix_web::dev::Service<
        actix_http::Request,
        Response = ServiceResponse,
        Error = actix_web::Error,
    > {
        init_service(
            App::new()
                .app_data(web::Data::new(metrics::ActiveUploads::new().unwrap()))
                .app_data(web::Data::new(metrics::StartedUploads::new().unwrap()))
                .app_data(web::Data::new(metrics::FinishedUploads::new().unwrap()))
                .app_data(web::Data::new(metrics::TerminatedUploads::new().unwrap()))
                .app_data(web::Data::new(metrics::UploadSizes::new().unwrap()))
                .configure(rustus_service(state.clone())),
        )
        .await
    }
}
