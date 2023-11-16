use std::{
    collections::HashMap,
    hash::{BuildHasher, BuildHasherDefault, DefaultHasher},
    str::FromStr,
};

use axum::http::HeaderMap;
use base64::{engine::general_purpose, Engine};
use rustc_hash::{FxHashMap, FxHasher};

static DISPOSITION_TYPE_INLINE: &str = "inline";
static DISPOSITION_TYPE_ATTACHMENT: &str = "attachment";

type Header = (axum::http::header::HeaderName, String);

pub trait HeaderMapExt {
    fn parse<T: FromStr>(&self, name: &str) -> Option<T>;
    fn check(&self, name: &str, expr: fn(&str) -> bool) -> bool;
    fn get_metadata(&self) -> Option<FxHashMap<String, String>>;
    fn get_upload_parts(&self) -> Vec<String>;
}

impl HeaderMapExt for HeaderMap {
    fn parse<T: FromStr>(&self, name: &str) -> Option<T> {
        self.get(name)?.to_str().ok()?.parse().ok()
    }

    fn check(&self, name: &str, expr: fn(&str) -> bool) -> bool {
        self.get(name)
            .and_then(|val| match val.to_str() {
                Ok(val) => Some(expr(val)),
                Err(_) => None,
            })
            .unwrap_or(false)
    }

    fn get_metadata(&self) -> Option<FxHashMap<String, String>> {
        let meta_split = self.get("Upload-Metadata")?.to_str().ok()?.split(',');
        let (shint, _) = meta_split.size_hint();
        let mut meta_map =
            HashMap::with_capacity_and_hasher(shint, BuildHasherDefault::<FxHasher>::default());
        for meta_entry in meta_split {
            let mut kval = meta_entry.trim().split(' ');
            let key = kval.next();
            let val = kval.next();
            if key.is_none() || val.is_none() {
                continue;
            }
            let value = general_purpose::STANDARD
                .decode(val.unwrap())
                .ok()
                .and_then(|val| String::from_utf8(val).ok());
            if let Some(value) = value {
                meta_map.insert(key.unwrap().to_string(), value);
            }
        }
        Some(meta_map)
    }

    fn get_upload_parts(&self) -> Vec<String> {
        self.get("Upload-Concat")
            .and_then(|header| header.to_str().ok())
            .and_then(|header| header.strip_prefix("final;"))
            .map(|urls| {
                urls.split(' ')
                    .filter_map(|val: &str| val.trim().split('/').last().map(String::from))
                    .filter(|val| val.trim() != "")
                    .collect()
            })
            .unwrap_or_default()
    }
}

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
