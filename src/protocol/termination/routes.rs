use actix_web::dev::HttpResponseBuilder;
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse};

use crate::errors::TuserResult;
use crate::Storage;

/// Terminate uploading.
///
/// This method will remove all
/// files by id.
pub async fn terminate<S: Storage>(
    storage: web::Data<S>,
    request: HttpRequest,
) -> TuserResult<HttpResponse> {
    let file_id_opt = request.match_info().get("file_id").map(String::from);
    if let Some(file_id) = file_id_opt {
        storage.remove_file(file_id.as_str()).await?;
    }
    Ok(HttpResponseBuilder::new(StatusCode::NO_CONTENT).body(""))
}
