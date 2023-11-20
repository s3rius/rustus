use std::{str::FromStr, time::Duration};

use http::{HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer, MaxAge};
use wildmatch::WildMatch;

pub fn layer(origins: Vec<String>, additional_headers: &[String]) -> CorsLayer {
    let mut allow_headers = additional_headers
        .iter()
        .filter_map(|header| HeaderName::from_str(header).ok())
        .collect::<Vec<_>>();
    allow_headers.extend_from_slice(&[
        HeaderName::from_static("content-type"),
        HeaderName::from_static("upload-offset"),
        HeaderName::from_static("upload-checksum"),
        HeaderName::from_static("upload-length"),
        HeaderName::from_static("upload-metadata"),
        HeaderName::from_static("upload-concat"),
        HeaderName::from_static("upload-defer-length"),
        HeaderName::from_static("tus-resumable"),
        HeaderName::from_static("tus-version"),
        HeaderName::from_static("x-http-method-override"),
        HeaderName::from_static("authorization"),
        HeaderName::from_static("origin"),
        HeaderName::from_static("x-requested-with"),
        HeaderName::from_static("x-request-id"),
        HeaderName::from_static("x-http-method-override"),
    ]);
    let mut cors = tower_http::cors::CorsLayer::new()
        .allow_methods([
            Method::OPTIONS,
            Method::GET,
            Method::HEAD,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers(allow_headers)
        .expose_headers(vec![
            HeaderName::from_static("location"),
            HeaderName::from_static("tus-version"),
            HeaderName::from_static("tus-resumable"),
            HeaderName::from_static("tus-max-size"),
            HeaderName::from_static("tus-extension"),
            HeaderName::from_static("tus-checksum-algorithm"),
            HeaderName::from_static("content-type"),
            HeaderName::from_static("content-length"),
            HeaderName::from_static("upload-length"),
            HeaderName::from_static("upload-metadata"),
            HeaderName::from_static("upload-defer-length"),
            HeaderName::from_static("upload-concat"),
            HeaderName::from_static("upload-offset"),
        ])
        .max_age(MaxAge::exact(Duration::from_secs(86400)));

    if origins.is_empty() {
        cors = cors.allow_origin(AllowOrigin::any());
    } else {
        cors = cors.allow_origin(AllowOrigin::predicate(
            move |request_origin: &HeaderValue, _| {
                for origin in &origins {
                    if WildMatch::new(origin) == request_origin.to_str().unwrap_or_default() {
                        return true;
                    }
                }
                false
            },
        ));
    }

    cors
}
