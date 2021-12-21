use crate::info_storages::FileInfo;
use actix_web::HttpRequest;

pub struct Message<'a> {
    request: &'a HttpRequest,
    file_info: FileInfo,
}
