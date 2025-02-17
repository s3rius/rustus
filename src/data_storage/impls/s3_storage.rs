use std::collections::HashMap;

use actix_web::{HttpRequest, HttpResponse, HttpResponseBuilder};
use chrono::{DateTime, Utc};
use s3::{
    command::Command,
    request::{tokio_backend::HyperRequest, Request},
    serde_types::Part,
    Bucket,
};
use serde::{Deserialize, Serialize};

use crate::{
    data_storage::base::DataStorage,
    errors::{RustusError, RustusResult},
    file_info::FileInfo,
    utils::{dir_struct::substr_time, headers::generate_disposition},
};

const UPLOAD_ID_KEY: &str = "_s3_upload_id";
const PART_NUMBER_KEY: &str = "_s3_chunk_number";
const PARTS_KEY: &str = "_s3_parts";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3MPUPart {
    pub part_number: u32,
    pub etag: String,
}

#[derive(Debug, Clone)]
pub struct S3DataStorage {
    bucket: Bucket,
    dir_struct: String,
}

impl From<S3MPUPart> for Part {
    fn from(value: S3MPUPart) -> Self {
        Self {
            part_number: value.part_number,
            etag: value.etag,
        }
    }
}

impl From<Part> for S3MPUPart {
    fn from(value: Part) -> Self {
        Self {
            part_number: value.part_number,
            etag: value.etag,
        }
    }
}

impl S3DataStorage {
    pub fn new(
        endpoint: String,
        region: String,
        access_key: Option<&String>,
        secret_key: Option<&String>,
        security_token: Option<&String>,
        session_token: Option<&String>,
        profile: Option<&String>,
        custom_headers: Option<&String>,
        bucket_name: &str,
        force_path_style: bool,
        dir_struct: String,
    ) -> Self {
        let creds = s3::creds::Credentials::new(
            access_key.map(String::as_str),
            secret_key.map(String::as_str),
            security_token.map(String::as_str),
            session_token.map(String::as_str),
            profile.map(String::as_str),
        );
        if let Err(err) = creds {
            panic!("Cannot build credentials: {err}")
        }
        log::debug!("Parsed credentials");
        let credentials = creds.unwrap();
        let bucket = Bucket::new(
            bucket_name,
            s3::Region::Custom { region, endpoint },
            credentials,
        );
        if let Err(error) = bucket {
            panic!("Cannot create bucket instance {error}");
        }
        let mut bucket = bucket.unwrap();
        if let Some(raw_s3_headers) = custom_headers {
            let headers_map = serde_json::from_str::<HashMap<String, String>>(raw_s3_headers)
                .expect("Cannot parse s3 headers. Please provide valid JSON object.");
            log::debug!("Found extra s3 headers.");
            for (key, value) in &headers_map {
                log::debug!("Adding header `{key}` with value `{value}`.");
                bucket.add_header(key, value);
            }
        }

        if force_path_style {
            bucket = bucket.with_path_style();
        }

        Self {
            bucket: *bucket,
            dir_struct,
        }
    }

    // Construct an S3 key which is used to upload files.
    fn get_s3_key(&self, id: &str, created_at: DateTime<Utc>) -> String {
        let base_path = substr_time(self.dir_struct.as_str(), created_at);
        let trimmed_path = base_path.trim_end_matches('/');
        format!("{trimmed_path}/{}", id)
    }
}

impl DataStorage for S3DataStorage {
    fn get_name(&self) -> &'static str {
        "s3"
    }

    async fn prepare(&mut self) -> crate::errors::RustusResult<()> {
        Ok(())
    }

    async fn get_contents(
        &self,
        file_info: &FileInfo,
        _request: &HttpRequest,
    ) -> RustusResult<HttpResponse> {
        let key = self.get_s3_key(&file_info.id, file_info.created_at);
        let command = Command::GetObject;
        let s3_request = HyperRequest::new(&self.bucket, &key, command).await?;
        let s3_response = s3_request.response_data_to_stream().await?;
        let mut response = HttpResponseBuilder::new(actix_web::http::StatusCode::OK);
        Ok(response
            .insert_header(generate_disposition(file_info.get_filename()))
            .streaming(s3_response.bytes))
    }

    async fn add_bytes(
        &self,
        file_info: &mut FileInfo,
        bytes: bytes::Bytes,
    ) -> crate::errors::RustusResult<()> {
        let s3_path = self.get_s3_key(&file_info.id, file_info.created_at);
        let part_num: u32 = file_info
            .metadata
            .entry(PART_NUMBER_KEY.to_string())
            .or_insert(String::from("1"))
            .parse()?;
        let mut parts: Vec<S3MPUPart> = serde_json::from_str(
            file_info
                .metadata
                .entry(PARTS_KEY.to_string())
                .or_insert(String::from("[]")),
        )?;
        let upload_id = file_info
            .metadata
            .get(UPLOAD_ID_KEY)
            .ok_or(RustusError::S3UploadIdMissing)?;
        log::debug!(
            "UPLOADING PART: {part_num} for {file_id}",
            part_num = part_num,
            file_id = file_info.id
        );
        let resp = self
            .bucket
            .put_multipart_chunk(bytes.to_vec(), &s3_path, part_num, &upload_id, "")
            .await?;
        parts.push(resp.into());
        if Some(file_info.offset + bytes.len()) == file_info.length {
            self.bucket
                .complete_multipart_upload(
                    &s3_path,
                    &upload_id,
                    parts.iter().cloned().map(Part::from).collect(),
                )
                .await?;
            file_info.metadata.remove(PARTS_KEY);
            file_info.metadata.remove(PART_NUMBER_KEY);
            file_info.metadata.remove(UPLOAD_ID_KEY);
            return Ok(());
        }
        file_info
            .metadata
            .insert(PART_NUMBER_KEY.to_string(), (part_num + 1).to_string());
        file_info
            .metadata
            .insert(PARTS_KEY.to_string(), serde_json::to_string(&parts)?);
        Ok(())
    }

    async fn create_file(&self, file_info: &mut FileInfo) -> crate::errors::RustusResult<String> {
        let s3_path = self.get_s3_key(&file_info.id, file_info.created_at);
        let resp = self.bucket.initiate_multipart_upload(&s3_path, "").await?;
        log::debug!("Created multipart upload with id: {}", resp.upload_id);
        file_info
            .metadata
            .insert(UPLOAD_ID_KEY.to_string(), resp.upload_id);
        Ok(s3_path)
    }

    async fn concat_files(
        &self,
        _file_info: &crate::file_info::FileInfo,
        _parts_info: Vec<crate::file_info::FileInfo>,
    ) -> crate::errors::RustusResult<()> {
        Err(RustusError::Unimplemented(
            "Concatenation is not supported for S3 storage.".into(),
        ))
    }

    async fn remove_file(
        &self,
        file_info: &crate::file_info::FileInfo,
    ) -> crate::errors::RustusResult<()> {
        if Some(file_info.offset) == file_info.length {
            self.bucket
                .delete_object(self.get_s3_key(&file_info.id, file_info.created_at))
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use s3::error::S3Error;

    use crate::data_storage::base::DataStorage;

    use super::S3DataStorage;

    fn get_s3_storage() -> S3DataStorage {
        let endpoint =
            std::env::var("TEST_S3_ENDPOINT").unwrap_or(String::from("http://localhost:9000"));
        let region = std::env::var("TEST_S3_REGION").unwrap_or(String::from("eu-west-1"));
        let access_key = std::env::var("TEST_S3_ACCESS_KEY").unwrap_or(String::from("rustus-test"));
        let secret_key = std::env::var("TEST_S3_SECRET_KEY").unwrap_or(String::from("rustus-test"));
        let bucket = std::env::var("TEST_S3_BUCKET").unwrap_or(String::from("rustus"));
        let path_style = env::var("TEST_S3_FORCE_PATH_STYLE")
            .unwrap_or(String::from("true"))
            .parse()
            .unwrap();
        S3DataStorage::new(
            endpoint,
            region,
            Some(&access_key),
            Some(&secret_key),
            None,
            None,
            None,
            None,
            &bucket,
            path_style,
            "".to_string(),
        )
    }

    #[actix_rt::test]
    async fn test_successfull_create_upload() {
        let storage = get_s3_storage();
        let data = "Hello World".as_bytes();
        let mut file_info = crate::file_info::FileInfo::new(
            &uuid::Uuid::new_v4().to_string(),
            Some(data.len()),
            None,
            "s3".to_string(),
            None,
        );
        let s3_path = storage.create_file(&mut file_info).await.unwrap();
        let resp = storage.bucket.get_object(s3_path).await.unwrap_err();
        match resp {
            S3Error::HttpFailWithBody(404, _) => {}
            _ => panic!("Unexpected error: {resp}"),
        }
        let ups = storage
            .bucket
            .list_multiparts_uploads(
                Some(&storage.get_s3_key(&file_info.id, file_info.created_at)),
                None,
            )
            .await
            .unwrap();
        assert_eq!(ups.len(), 1);
    }

    #[actix_rt::test]
    async fn test_successfull_upload() {
        let storage = get_s3_storage();
        let data = "Hello World".as_bytes();
        let mut file_info = crate::file_info::FileInfo::new(
            &uuid::Uuid::new_v4().to_string(),
            Some(data.len()),
            None,
            "s3".to_string(),
            None,
        );
        let s3_path = storage.create_file(&mut file_info).await.unwrap();
        storage
            .add_bytes(&mut file_info, data.into())
            .await
            .unwrap();
        let object = storage.bucket.get_object(s3_path).await.unwrap();
        assert_eq!(object.bytes(), data);
    }
}
