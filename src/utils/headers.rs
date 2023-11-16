static DISPOSITION_TYPE_INLINE: &str = "inline";
static DISPOSITION_TYPE_ATTACHMENT: &str = "attachment";

type Header = (axum::http::header::HeaderName, String);

pub fn generate_disposition(filename: &str) -> (Header, Header) {
    let mime_type = mime_guess::from_path(filename).first_or_octet_stream();

    let disposition = match mime_type.type_() {
        mime::IMAGE | mime::TEXT | mime::AUDIO | mime::VIDEO => DISPOSITION_TYPE_INLINE,
        mime::APPLICATION => match mime_type.subtype() {
            mime::JAVASCRIPT | mime::JSON => DISPOSITION_TYPE_INLINE,
            name if name == "wasm" => DISPOSITION_TYPE_INLINE,
            _ => DISPOSITION_TYPE_ATTACHMENT,
        },
        _ => DISPOSITION_TYPE_ATTACHMENT,
    };

    return (
        (
            axum::http::header::CONTENT_DISPOSITION,
            format!("{}; filename=\"{}\"", disposition, filename),
        ),
        (axum::http::header::CONTENT_TYPE, mime_type.to_string()),
    );
}
