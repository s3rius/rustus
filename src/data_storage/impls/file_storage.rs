use std::path::PathBuf;

use tokio::io::AsyncWriteExt;

use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use std::fs::DirBuilder;

use crate::{
    data_storage::base::Storage,
    errors::{RustusError, RustusResult},
    models::file_info::FileInfo,
    utils::{dir_struct::substr_now, headers::HeaderMapExt},
};

#[derive(Clone, Debug)]
pub struct FileStorage {
    data_dir: PathBuf,
    dir_struct: String,
    force_fsync: bool,
}

impl FileStorage {
    #[must_use]
    pub const fn new(data_dir: PathBuf, dir_struct: String, force_fsync: bool) -> Self {
        Self {
            data_dir,
            dir_struct,
            force_fsync,
        }
    }

    /// Create path to file in a data directory.
    ///
    /// This function is using template from `dir_struct` field
    /// and based on it creates path to file.
    ///
    /// # Errors
    ///
    /// Might retur an error, if path is invalid, or directory cannot be created.
    pub fn data_file_path(&self, file_id: &str) -> RustusResult<PathBuf> {
        let dir = self
            .data_dir
            // We're working wit absolute paths, because tus.io says so.
            .canonicalize()?
            .join(substr_now(self.dir_struct.as_str()));
        DirBuilder::new().recursive(true).create(dir.as_path())?;
        Ok(dir.join(file_id))
    }
}

impl Storage for FileStorage {
    fn get_name(&self) -> &'static str {
        "file"
    }

    async fn prepare(&mut self) -> RustusResult<()> {
        // We're creating directory for new files
        // if it doesn't already exist.
        if !self.data_dir.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(self.data_dir.as_path())?;
        }
        Ok(())
    }

    async fn get_contents(&self, file_info: &FileInfo) -> RustusResult<Response> {
        if file_info.path.is_none() {
            return Err(RustusError::FileNotFound);
        };
        let file = tokio::fs::File::open(file_info.path.clone().unwrap().as_str())
            .await
            .map_err(|_| RustusError::FileNotFound)?;
        let buf_file = tokio::io::BufReader::new(file);
        let reader = tokio_util::io::ReaderStream::new(buf_file);
        let mut resp = axum::body::Body::from_stream(reader).into_response();
        resp.headers_mut()
            .generate_disposition(file_info.get_filename());
        Ok(resp)
    }

    async fn add_bytes(&self, file_info: &FileInfo, bytes: Bytes) -> RustusResult<()> {
        // In normal situation this `if` statement is not
        // gonna be called, but what if it is ...
        let Some(path) = &file_info.path else {
            return Err(RustusError::FileNotFound);
        };
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(false)
            .read(false)
            .truncate(false)
            .open(path.as_str())
            .await?;
        let mut writer = tokio::io::BufWriter::new(file);
        writer.write_all(&bytes).await?;
        writer.flush().await?;
        if self.force_fsync {
            writer.get_ref().sync_data().await?;
        }
        writer.into_inner().shutdown().await?;
        Ok(())
    }

    async fn create_file(&self, file_info: &FileInfo) -> RustusResult<String> {
        // New path to file.
        let file_path = self.data_file_path(file_info.id.as_str())?;
        let mut opened = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .create_new(true)
            .open(file_path.as_path())
            .await?;
        opened.shutdown().await?;
        Ok(file_path.display().to_string())
    }

    async fn concat_files(
        &self,
        file_info: &FileInfo,
        parts_info: Vec<FileInfo>,
    ) -> RustusResult<()> {
        let Some(path) = &file_info.path else {
            return Err(RustusError::FileNotFound);
        };
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(path)
            .await?;
        let mut writer = tokio::io::BufWriter::new(file);
        for part in parts_info {
            if part.path.is_none() {
                return Err(RustusError::FileNotFound);
            }
            let part_file = tokio::fs::OpenOptions::new()
                .read(true)
                .open(part.path.as_ref().unwrap())
                .await?;
            let mut reader = tokio::io::BufReader::new(part_file);
            tokio::io::copy_buf(&mut reader, &mut writer).await?;
            reader.shutdown().await?;
        }
        writer.flush().await?;
        let mut inner_file = writer.into_inner();
        if self.force_fsync {
            inner_file.sync_data().await?;
        }
        inner_file.shutdown().await?;
        Ok(())
    }

    async fn remove_file(&self, file_info: &FileInfo) -> RustusResult<()> {
        let Some(path) = &file_info.path else {
            return Err(RustusError::FileNotFound);
        };
        let data_path = PathBuf::from(path);
        if !data_path.exists() {
            return Err(RustusError::FileNotFound);
        }
        tokio::fs::remove_file(data_path).await.map_err(|err| {
            tracing::error!("{:?}", err);
            RustusError::UnableToRemove(String::from(path.as_str()))
        })?;
        Ok(())
    }
}
