use std::{collections::HashMap, io::Write};

use actix_web::{HttpRequest, HttpResponse, HttpResponseBuilder};
use chrono::{DateTime, Utc};
use futures::{StreamExt, TryStreamExt};
use s3::{
    command::Command,
    request::{tokio_backend::HyperRequest, Request},
    serde_types::Part,
    Bucket,
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::{
    data_storage::base::DataStorage,
    errors::{RustusError, RustusResult},
    file_info::FileInfo,
    utils::{dir_struct::substr_time, headers::generate_disposition},
};

const UPLOAD_ID_KEY: &str = "_s3_upload_id";
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
    concurrent_concat_downloads: usize,
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
    #[allow(clippy::too_many_arguments)]
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
        concat_concurrent_downloads: usize,
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
            concurrent_concat_downloads: concat_concurrent_downloads,
        }
    }

    // Construct an S3 key which is used to upload files.
    fn get_s3_key(&self, id: &str, created_at: DateTime<Utc>) -> String {
        let base_path = substr_time(self.dir_struct.as_str(), created_at);
        let trimmed_path = base_path.trim_end_matches('/');
        format!("{trimmed_path}/{id}")
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
        let mut parts: Vec<S3MPUPart> = serde_json::from_str(
            file_info
                .metadata
                .entry(PARTS_KEY.to_string())
                .or_insert_with(|| String::from("[]")),
        )?;
        let upload_id = file_info
            .metadata
            .get(UPLOAD_ID_KEY)
            .ok_or(RustusError::S3UploadIdMissing)?;
        log::debug!(
            "UPLOADING PART: {part_num} for {file_id}",
            part_num = parts.len() + 1,
            file_id = file_info.id
        );
        let content_type = mime_guess::from_path(file_info.get_filename()).first_or_octet_stream();
        let resp = self
            .bucket
            .put_multipart_chunk(
                bytes.to_vec(),
                &s3_path,
                u32::try_from(parts.len() + 1)?,
                upload_id,
                content_type.as_ref(),
            )
            .await?;
        parts.push(resp.into());
        if Some(file_info.offset + bytes.len()) == file_info.length {
            self.bucket
                .complete_multipart_upload(
                    &s3_path,
                    upload_id,
                    parts.iter().cloned().map(Part::from).collect(),
                )
                .await?;
            file_info.metadata.remove(PARTS_KEY);
            file_info.metadata.remove(UPLOAD_ID_KEY);
            return Ok(());
        }
        file_info
            .metadata
            .insert(PARTS_KEY.to_string(), serde_json::to_string(&parts)?);
        Ok(())
    }

    async fn create_file(&self, file_info: &mut FileInfo) -> crate::errors::RustusResult<String> {
        let s3_path = self.get_s3_key(&file_info.id, file_info.created_at);
        let mime_type = mime_guess::from_path(file_info.get_filename()).first_or_octet_stream();
        let resp = self
            .bucket
            .initiate_multipart_upload(&s3_path, mime_type.as_ref())
            .await?;
        log::debug!("Created multipart upload with id: {}", resp.upload_id);
        file_info
            .metadata
            .insert(UPLOAD_ID_KEY.to_string(), resp.upload_id);
        Ok(s3_path)
    }

    async fn concat_files(
        &self,
        file_info: &crate::file_info::FileInfo,
        parts_info: Vec<crate::file_info::FileInfo>,
    ) -> crate::errors::RustusResult<()> {
        let dir = tempdir::TempDir::new(&file_info.id)?;
        let mut download_futures = vec![];

        // At first we need to download all parts.
        for part_info in &parts_info {
            let part_key = self.get_s3_key(&part_info.id, part_info.created_at);
            let part_out = dir.path().join(&part_info.id);
            // Here we create a future which downloads the part
            // into a temporary file.
            download_futures.push(async move {
                let part_file = tokio::fs::File::create(&part_out).await?;
                let mut writer = tokio::io::BufWriter::new(part_file);
                let mut reader = self.bucket.get_object_stream(&part_key).await?;
                while let Some(chunk) = reader.bytes().next().await {
                    let mut chunk = chunk?;
                    writer.write_all_buf(&mut chunk).await.map_err(|err| {
                        log::error!("{:?}", err);
                        RustusError::UnableToWrite(err.to_string())
                    })?;
                }
                writer.flush().await?;
                writer.get_ref().sync_data().await?;
                Ok::<_, RustusError>(())
            });
        }
        // Here we await all download futures.
        // We use buffer_unordered to limit the number of concurrent downloads.
        futures::stream::iter(download_futures)
            // Number of concurrent downloads.
            .buffer_unordered(self.concurrent_concat_downloads)
            // We use try_collect to collect all results
            // and return an error if any of the futures returned an error.
            .try_collect::<Vec<_>>()
            .await?;

        let output_path = dir.path().join(&file_info.id);
        let output_path_cloned = output_path.clone();
        let parts_files = parts_info
            .iter()
            .map(|info| dir.path().join(&info.id))
            .collect::<Vec<_>>();
        tokio::task::spawn_blocking(move || {
            let file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(output_path_cloned)
                .map_err(|err| {
                    log::error!("{:?}", err);
                    RustusError::UnableToWrite(err.to_string())
                })?;
            let mut writer = std::io::BufWriter::new(file);
            for part in &parts_files {
                let part_file = std::fs::OpenOptions::new().read(true).open(part)?;
                let mut reader = std::io::BufReader::new(part_file);
                std::io::copy(&mut reader, &mut writer)?;
            }
            writer.flush()?;
            writer.get_ref().sync_data()?;
            Ok::<_, RustusError>(())
        })
        .await??;

        // We reopen the file to upload it to S3.
        // This is needed because we need to open the file in read mode.
        let output_file = tokio::fs::File::open(&output_path).await?;
        let mut reader = tokio::io::BufReader::new(output_file);
        let key = self.get_s3_key(&file_info.id, file_info.created_at);
        self.bucket.put_object_stream(&mut reader, key).await?;

        tokio::fs::remove_file(output_path).await?;

        Ok(())
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
            4,
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
            storage.get_name().to_string(),
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
            storage.get_name().to_string(),
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

    #[actix_rt::test]
    async fn test_successfull_delete() {
        let storage = get_s3_storage();
        let data = "Hello World".as_bytes();
        let mut file_info = crate::file_info::FileInfo::new(
            &uuid::Uuid::new_v4().to_string(),
            Some(data.len()),
            None,
            storage.get_name().to_string(),
            None,
        );
        let s3_path = storage.create_file(&mut file_info).await.unwrap();
        storage
            .add_bytes(&mut file_info, data.into())
            .await
            .unwrap();
        file_info.offset += data.len();
        let object = storage.bucket.get_object(s3_path.clone()).await.unwrap();
        assert_eq!(object.bytes(), data);
        storage.remove_file(&file_info).await.unwrap();
        let resp = storage.bucket.get_object(s3_path).await.unwrap_err();
        match resp {
            S3Error::HttpFailWithBody(404, _) => {}
            _ => panic!("Unexpected error: {resp}"),
        }
    }

    #[actix_rt::test]
    async fn test_successfull_mime() {
        let storage = get_s3_storage();
        let data = "Helloworld of videos!".as_bytes();
        let mut file_info = crate::file_info::FileInfo::new(
            &uuid::Uuid::new_v4().to_string(),
            Some(data.len()),
            None,
            storage.get_name().to_string(),
            None,
        );
        file_info
            .metadata
            .insert(String::from("filename"), String::from("meme.mp4"));
        let s3_path = storage.create_file(&mut file_info).await.unwrap();
        storage
            .add_bytes(&mut file_info, data.into())
            .await
            .unwrap();
        let object = storage.bucket.get_object(s3_path).await.unwrap();
        assert_eq!(object.bytes(), data);
        assert_eq!(
            object.headers().get("content-type"),
            Some(&String::from("video/mp4"))
        )
    }

    #[actix_rt::test]
    async fn test_successfull_concat() {
        let storage = get_s3_storage();

        let fst_data = "Hello".as_bytes();
        let mut fst_file_info = crate::file_info::FileInfo::new(
            &uuid::Uuid::new_v4().to_string(),
            Some(fst_data.len()),
            None,
            storage.get_name().to_string(),
            None,
        );
        fst_file_info.is_partial = true;
        let snd_data = "World".as_bytes();
        let mut snd_file_info = crate::file_info::FileInfo::new(
            &uuid::Uuid::new_v4().to_string(),
            Some(snd_data.len()),
            None,
            storage.get_name().to_string(),
            None,
        );
        snd_file_info.is_partial = true;
        let fst_s3_path = storage.create_file(&mut fst_file_info).await.unwrap();
        let snd_s3_path = storage.create_file(&mut snd_file_info).await.unwrap();
        storage
            .add_bytes(&mut fst_file_info, fst_data.into())
            .await
            .unwrap();
        storage
            .add_bytes(&mut snd_file_info, snd_data.into())
            .await
            .unwrap();

        let fst_object = storage.bucket.get_object(&fst_s3_path).await.unwrap();
        assert_eq!(fst_object.bytes(), fst_data);
        let snd_object = storage.bucket.get_object(&snd_s3_path).await.unwrap();
        assert_eq!(snd_object.bytes(), snd_data);

        let mut final_file_info = crate::file_info::FileInfo::new(
            &uuid::Uuid::new_v4().to_string(),
            Some(fst_data.len() + snd_data.len()),
            None,
            storage.get_name().to_string(),
            None,
        );
        final_file_info.is_final = true;
        storage
            .concat_files(&final_file_info, vec![fst_file_info, snd_file_info])
            .await
            .unwrap();
        let final_s3_path = storage.get_s3_key(&final_file_info.id, final_file_info.created_at);
        let object = storage.bucket.get_object(&final_s3_path).await.unwrap();
        assert_eq!(object.bytes(), b"HelloWorld".as_slice());
    }
}
