use std::{collections::HashMap, hash::BuildHasherDefault, net::SocketAddr};

use http::{HeaderMap, Uri};
use rustc_hash::FxHasher;
use serde_json::{json, Value};

use crate::{models::file_info::FileInfo, utils::headers::HeaderMapExt};

#[derive(Clone, Debug, Eq, strum::Display, PartialEq, strum::EnumIter)]
pub enum Format {
    #[strum(serialize = "default")]
    Default,
    #[strum(serialize = "tusd")]
    Tusd,
    #[strum(serialize = "v2")]
    V2,
}

pub struct HookData<'a> {
    pub uri: String,
    pub method: &'a str,
    pub remote_addr: String,
    pub headers: &'a HeaderMap,

    pub file_info: &'a FileInfo,
}

impl Format {
    pub fn format(
        &self,
        uri: &Uri,
        method: &http::Method,
        addr: &SocketAddr,
        headers: &HeaderMap,
        proxy_enabled: bool,
        file_info: &FileInfo,
    ) -> String {
        let hook_data = &HookData::new(
            uri.path_and_query()
                .map(ToString::to_string)
                .unwrap_or_default(),
            method.as_str(),
            headers.get_remote_ip(addr, proxy_enabled),
            headers,
            file_info,
        );
        match self {
            Format::Default => default_format(hook_data),
            Format::Tusd => tusd_format(hook_data),
            Format::V2 => v2_format(hook_data),
        }
    }
}

impl<'a> HookData<'a> {
    #[must_use]
    pub fn new(
        uri: String,
        method: &'a str,
        remote_addr: String,
        headers: &'a HeaderMap,
        file_info: &'a FileInfo,
    ) -> Self {
        Self {
            uri,
            method,
            remote_addr,
            headers,
            file_info,
        }
    }
}

crate::from_str!(Format, "format");
fn headers_to_value_map(
    headers: &HeaderMap,
    use_arrays: bool,
) -> HashMap<String, Value, BuildHasherDefault<FxHasher>> {
    let mut headers_map =
        HashMap::with_capacity_and_hasher(headers.len(), BuildHasherDefault::<FxHasher>::default());
    for (name, value) in headers {
        if let Ok(header_val) = value.to_str().map(String::from) {
            if use_arrays {
                headers_map.insert(
                    name.to_string(),
                    Value::Array(vec![Value::String(header_val)]),
                );
            } else {
                headers_map.insert(name.to_string(), Value::String(header_val));
            }
        }
    }
    headers_map
}

#[must_use]
pub fn default_format(hook_data: &HookData) -> String {
    json!({
        "upload": {
            "id": hook_data.file_info.id,
            "offset": hook_data.file_info.offset,
            "length": hook_data.file_info.length,
            "path": hook_data.file_info.path,
            "created_at": hook_data.file_info.created_at.timestamp(),
            "deferred_size": hook_data.file_info.deferred_size,
            "is_partial": hook_data.file_info.is_partial,
            "is_final": hook_data.file_info.is_final,
            "metadata": hook_data.file_info.metadata,
            "storage": hook_data.file_info.storage,
            "parts": hook_data.file_info.parts,
        },
        "request": {
            "URI": hook_data.uri,
            "method": hook_data.method,
            "remote_addr": hook_data.remote_addr,
            "headers": headers_to_value_map(hook_data.headers , false)
        }
    })
    .to_string()
}

#[must_use]
pub fn v2_format(hook_data: &HookData) -> String {
    json!({
        "upload": {
            "id": hook_data.file_info.id,
            "offset": hook_data.file_info.offset,
            "length": hook_data.file_info.length,
            "path": hook_data.file_info.path,
            "created_at": hook_data.file_info.created_at.timestamp(),
            "deferred_size": hook_data.file_info.deferred_size,
            "is_partial": hook_data.file_info.is_partial,
            "is_final": hook_data.file_info.is_final,
            "metadata": hook_data.file_info.metadata,
            "storage": hook_data.file_info.storage,
            "parts": hook_data.file_info.parts,
        },
        "request": {
            "uri": hook_data.uri,
            "method": hook_data.method,
            "remote_addr": hook_data.remote_addr,
            "headers": headers_to_value_map(hook_data.headers , false)
        }
    })
    .to_string()
}

#[must_use]
pub fn tusd_format(hook_data: &HookData) -> String {
    json!({
        "upload": {
            "ID": hook_data.file_info.id,
            "Offset": hook_data.file_info.offset,
            "Size": hook_data.file_info.length,
            "CreatedAt": hook_data.file_info.created_at.timestamp(),
            "SizeIsDeferred": hook_data.file_info.deferred_size,
            "IsPartial": hook_data.file_info.is_partial,
            "IsFinal": hook_data.file_info.is_final,
            "MetaData": hook_data.file_info.metadata,
            "Storage": {
                "Type": hook_data.file_info.storage,
                "Path": hook_data.file_info.path,
            },
            "Parts": hook_data.file_info.parts,
        },
        "HTTPRequest": {
            "URI": hook_data.uri,
            "Method": hook_data.method,
            "RemoteAddr": hook_data.remote_addr,
            "Header": headers_to_value_map(hook_data.headers , true)
        }
    })
    .to_string()
}
