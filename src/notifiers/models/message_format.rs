use crate::{from_str, info_storages::FileInfo};
use actix_web::{http::header::HeaderMap, HttpRequest};
use derive_more::{Display, From};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use strum::EnumIter;

#[derive(Clone, Debug, Eq, Display, From, PartialEq, EnumIter)]
pub enum Format {
    #[display(fmt = "default")]
    Default,
    #[display(fmt = "tusd")]
    Tusd,
}

from_str!(Format, "format");

impl Format {
    pub fn format(&self, request: &HttpRequest, file_info: &FileInfo) -> String {
        match self {
            Self::Default => default_format(request, file_info),
            Self::Tusd => tusd_format(request, file_info),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct TusdStorageInfo {
    #[serde(rename = "Type")]
    storage_type: String,
    path: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct TusdFileInfo {
    #[serde(rename = "ID")]
    id: String,
    offset: usize,
    size: Option<usize>,
    is_final: bool,
    is_partial: bool,
    partial_uploads: Option<Vec<String>>,
    size_is_deferred: bool,
    metadata: HashMap<String, String>,
    storage: TusdStorageInfo,
}

impl From<FileInfo> for TusdFileInfo {
    fn from(file_info: FileInfo) -> Self {
        let deferred_size = file_info.length.is_none();
        Self {
            id: file_info.id,
            offset: file_info.offset,
            size: file_info.length,
            size_is_deferred: deferred_size,
            is_final: file_info.is_final,
            is_partial: file_info.is_partial,
            partial_uploads: file_info.parts,
            metadata: file_info.metadata,
            storage: TusdStorageInfo {
                storage_type: file_info.storage,
                path: file_info.path,
            },
        }
    }
}

/// Transforms headersmap to `HashMap`.
///
/// Keys of the resulting map are Strings,
/// Values are serde values. It ca be either string values or
/// arrays.
fn headers_to_value_map(headers: &HeaderMap, use_arrays: bool) -> HashMap<String, Value> {
    let mut headers_map = HashMap::new();
    for (name, value) in headers.iter() {
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

/// Default format is specific for Rustus.
///
/// This format is a simple serialized `FileInfo` and some parts of the request.
pub fn default_format(request: &HttpRequest, file_info: &FileInfo) -> String {
    let remote_addr = request.connection_info().peer_addr().map(String::from);
    let value = json!({
        "upload": file_info,
        "request": {
            "URI": request.uri().to_string(),
            "method": request.method().to_string(),
            "remote_addr": remote_addr,
            "headers": headers_to_value_map(request.headers(), false)
        }
    });
    value.to_string()
}

/// This format follows TUSD hooks.
///
/// You can read more about tusd hooks
/// [here](https://github.com/tus/tusd/blob/master/docs/hooks.md).
///
/// Generally speaking, it's almost the same as the default format,
/// but some variables are ommited and headers are added to the request.
pub fn tusd_format(request: &HttpRequest, file_info: &FileInfo) -> String {
    let remote_addr = request.connection_info().peer_addr().map(String::from);
    let value = json!({
        "Upload": file_info,
        "HTTPRequest": {
            "URI": request.uri().to_string(),
            "Method": request.method().to_string(),
            "RemoteAddr": remote_addr,
            "Header": headers_to_value_map(request.headers(), false)
        }
    });
    value.to_string()
}
