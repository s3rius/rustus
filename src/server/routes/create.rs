use bytes::Bytes;

use crate::errors::RustusResult;

pub async fn create_route(body: Bytes) -> RustusResult<()> {
    println!("{:?}", body);
    Ok(())
}
