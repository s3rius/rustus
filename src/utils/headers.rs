use std::str::FromStr;

use actix_web::HttpRequest;

/// Parse header's value.
///
/// This function will try to parse
/// header's value to some type T.
///
/// If header is not present or value
/// can't be parsed then it returns None.
pub fn parse_header<T: FromStr>(request: &HttpRequest, header_name: &str) -> Option<T> {
    request
        .headers()
        // Get header
        .get(header_name)
        .and_then(|value|
            // Parsing it to string.
            match value.to_str() {
                Ok(header_str) => Some(String::from(header_str)),
                Err(_) => None,
            })
        .and_then(|val|
            // Parsing to type T.
            match val.parse::<T>() {
                Ok(num) => Some(num),
                Err(_) => None,
            })
}

/// Check that header value satisfies some predicate.
///
/// Passes header as a parameter to expr if header is present.
pub fn check_header(request: &HttpRequest, header_name: &str, expr: fn(&str) -> bool) -> bool {
    request
        .headers()
        .get(header_name)
        .and_then(|header_val| match header_val.to_str() {
            Ok(val) => Some(expr(val)),
            Err(_) => None,
        })
        .unwrap_or(false)
}
