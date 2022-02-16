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

#[cfg(test)]
mod tests {
    use super::{check_header, parse_header};
    use actix_web::test::TestRequest;

    #[actix_rt::test]
    async fn test_parse_header_unknown_header() {
        let request = TestRequest::get().to_http_request();
        let header = parse_header::<String>(&request, "unknown");
        assert!(header.is_none());
    }

    #[actix_rt::test]
    async fn test_parse_header_wrong_type() {
        let request = TestRequest::get()
            .insert_header(("test_header", String::from("test").as_bytes()))
            .to_http_request();
        let header = parse_header::<i32>(&request, "test_header");
        assert!(header.is_none());
    }

    #[actix_rt::test]
    async fn test_parse_header() {
        let request = TestRequest::get()
            .insert_header(("test_header", String::from("123").as_bytes()))
            .to_http_request();
        let header = parse_header::<usize>(&request, "test_header");
        assert_eq!(header.unwrap(), 123);
    }

    #[actix_rt::test]
    async fn test_check_header_unknown_header() {
        let request = TestRequest::get().to_http_request();
        let check = check_header(&request, "unknown", |value| value == "1");
        assert_eq!(check, false);
    }

    #[actix_rt::test]
    async fn test_check_header() {
        let request = TestRequest::get()
            .insert_header(("test_header", "1"))
            .to_http_request();
        let check = check_header(&request, "test_header", |value| value == "1");
        assert!(check);
        let check = check_header(&request, "test_header", |value| value == "2");
        assert!(!check);
    }
}
