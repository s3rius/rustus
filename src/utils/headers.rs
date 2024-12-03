use std::str::FromStr;

use actix_web::{
    http::header::{ContentDisposition, DispositionParam, DispositionType},
    HttpRequest,
};

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
        // Parsing it to string.
        .and_then(|value| value.to_str().ok())
        // Parsing to type T.
        .and_then(|val| val.parse::<T>().ok())
}

/// Check that header value satisfies some predicate.
///
/// Passes header as a parameter to expr if header is present.
pub fn check_header(request: &HttpRequest, header_name: &str, expr: fn(&str) -> bool) -> bool {
    request
        .headers()
        .get(header_name)
        // Parsing it to string.
        .and_then(|header_val| header_val.to_str().ok())
        // Applying predicate.
        .is_some_and(expr)
}

/// This function generates content disposition
/// based on file name.
pub fn generate_disposition(filename: &str) -> ContentDisposition {
    let mime_type = mime_guess::from_path(filename).first_or_octet_stream();
    let disposition = match mime_type.type_() {
        mime::IMAGE | mime::TEXT | mime::AUDIO | mime::VIDEO => DispositionType::Inline,
        mime::APPLICATION => match mime_type.subtype() {
            mime::JAVASCRIPT | mime::JSON => DispositionType::Inline,
            name if name == "wasm" => DispositionType::Inline,
            _ => DispositionType::Attachment,
        },
        _ => DispositionType::Attachment,
    };

    ContentDisposition {
        disposition,
        parameters: vec![DispositionParam::Filename(String::from(filename))],
    }
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
        assert!(!check);
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
