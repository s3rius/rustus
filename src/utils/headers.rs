use std::{collections::HashMap, hash::BuildHasherDefault, net::SocketAddr, str::FromStr};

use axum::http::{HeaderMap, HeaderValue};
use base64::{engine::general_purpose, Engine};
use rustc_hash::{FxHashMap, FxHasher};

static DISPOSITION_TYPE_INLINE: &str = "inline";
static DISPOSITION_TYPE_ATTACHMENT: &str = "attachment";

pub trait HeaderMapExt {
    fn parse<T: FromStr>(&self, name: &str) -> Option<T>;
    fn check(&self, name: &str, expr: fn(&str) -> bool) -> bool;
    fn get_metadata(&self) -> Option<FxHashMap<String, String>>;
    fn get_upload_parts(&self) -> Vec<String>;
    fn get_method_override(&self) -> Option<axum::http::Method>;
    fn generate_disposition(&mut self, filename: &str);
    fn get_remote_ip(&self, socket_addr: &SocketAddr, proxy_enabled: bool) -> String;
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

    fn get_method_override(&self) -> Option<axum::http::Method> {
        self.get("X-HTTP-Method-Override")
            .and_then(|header| header.to_str().ok())
            .and_then(|header| header.trim().parse().ok())
    }

    fn generate_disposition(&mut self, filename: &str) {
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

        format!("{}; filename=\"{}\"", disposition, filename)
            .parse::<HeaderValue>()
            .map(|val| {
                self.insert(axum::http::header::CONTENT_DISPOSITION, val);
            })
            .ok();
        mime_type
            .to_string()
            .parse::<HeaderValue>()
            .map(|val| self.insert(axum::http::header::CONTENT_TYPE, val))
            .ok();
    }

    fn get_remote_ip(&self, socket_addr: &SocketAddr, proxy_enabled: bool) -> String {
        if !proxy_enabled {
            return socket_addr.ip().to_string();
        }
        self.get("Forwarded")
            .or_else(|| self.get("X-Forwarded-For"))
            .and_then(|val| val.to_str().ok())
            .map(|st| st.to_string())
            .unwrap_or_else(|| socket_addr.ip().to_string())
    }
}
