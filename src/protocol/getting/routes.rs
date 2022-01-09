use actix_web::{web, HttpRequest, Responder};

use crate::errors::RustusError;
use crate::{InfoStorage, Storage};

/// Retrieve actual file.
///
/// This method allows you to download files directly from storage.
pub async fn get_file(
    request: HttpRequest,
    storage: web::Data<Box<dyn Storage + Send + Sync>>,
    info_storage: web::Data<Box<dyn InfoStorage + Send + Sync>>,
) -> impl Responder {
    let file_id_opt = request.match_info().get("file_id").map(String::from);
    if let Some(file_id) = file_id_opt {
        let file_info = info_storage.get_info(file_id.as_str()).await?;
        if file_info.storage != storage.to_string() {
            return Err(RustusError::FileNotFound);
        }
        storage.get_contents(&file_info).await
    } else {
        Err(RustusError::FileNotFound)
    }
}
