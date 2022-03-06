use crate::{errors::RustusResult, info_storages::FileInfo};
use actix_web::HttpRequest;
use derive_more::{Display, From};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::from_str;
use strum::EnumIter;

#[derive(Clone, Debug, Eq, Display, From, PartialEq, EnumIter)]
pub enum Format {
    #[display(fmt = "default")]
    Default,
    #[display(fmt = "tusd")]
    Tusd,
    #[display(fmt = "celery")]
    Celery,
}

from_str!(Format, "format");

impl Format {
    pub fn format(&self, request: &HttpRequest, file_info: &FileInfo) -> RustusResult<String> {
        match self {
            Self::Default => default_format(request, file_info),
            Self::Tusd => tusd_format(request, file_info),
            Self::Celery => celery_format(request, file_info),
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

/// Turn request into `serde_json::Value`.
///
/// This function is used by different formats.
fn serialize_request(
    request: &HttpRequest,
    method_str: String,
    remote_addr_str: String,
    headers_str: String,
    use_arrays: bool,
) -> Value {
    let mut map = Map::new();
    map.insert("URI".into(), Value::String(request.uri().to_string()));
    map.insert(method_str, Value::String(request.method().to_string()));
    map.insert(
        remote_addr_str,
        Value::String(
            request
                .connection_info()
                .realip_remote_addr()
                .map_or_else(String::new, String::from),
        ),
    );
    let mut headers_map = Map::new();
    for (name, value) in request.headers() {
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
    map.insert(headers_str, Value::Object(headers_map));
    Value::Object(map)
}

/// Default format is specific for Rustus.
///
/// This format is a simple serialized `FileInfo` and some parts of the request.
pub fn default_format(request: &HttpRequest, file_info: &FileInfo) -> RustusResult<String> {
    let mut result_map = Map::new();
    result_map.insert("upload".into(), serde_json::to_value(file_info)?);
    result_map.insert(
        "request".into(),
        serialize_request(
            request,
            "method".into(),
            "remote_addr".into(),
            "headers".into(),
            false,
        ),
    );
    Ok(Value::Object(result_map).to_string())
}

/// This format follows TUSD hooks.
///
/// You can read more about tusd hooks
/// [here](https://github.com/tus/tusd/blob/master/docs/hooks.md).
///
/// Generally speaking, it's almost the same as the default format,
/// but some variables are ommited and headers are added to the request.
pub fn tusd_format(request: &HttpRequest, file_info: &FileInfo) -> RustusResult<String> {
    let mut result_map = Map::new();

    result_map.insert(
        "Upload".into(),
        serde_json::to_value(TusdFileInfo::from(file_info.clone()))?,
    );
    result_map.insert(
        "HTTPRequest".into(),
        serialize_request(
            request,
            "Method".into(),
            "RemoteAddr".into(),
            "Header".into(),
            true,
        ),
    );
    Ok(Value::Object(result_map).to_string())
}

pub fn celery_format(_request: &HttpRequest, _file_info: &FileInfo) -> RustusResult<String> {
    todo!()
}
