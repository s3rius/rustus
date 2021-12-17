use actix_web::{HttpRequest, Responder, web};

use crate::errors::TuserError;
use crate::Storage;

pub async fn get_file(
    request: HttpRequest,
    storage: web::Data<Box<dyn Storage + Send + Sync>>,
) -> impl Responder {
    let file_id_opt = request.match_info().get("file_id").map(String::from);
    if let Some(file_id) = file_id_opt {
        storage.get_contents(file_id.as_str()).await
    } else {
        Err(TuserError::FileNotFound)
    }
}
