use std::{io::Write, path::PathBuf};

use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use log::error;
use std::{
    fs::{remove_file, DirBuilder, OpenOptions},
    io::{copy, BufReader, BufWriter},
};

use crate::{
    errors::{RustusError, RustusResult},
    models::file_info::FileInfo,
    utils::{dir_struct::substr_now, headers::generate_disposition},
};

use super::base::Storage;

#[derive(Clone)]
pub struct FileStorage {
    data_dir: PathBuf,
    dir_struct: String,
    force_fsync: bool,
}

impl FileStorage {
    pub fn new(data_dir: PathBuf, dir_struct: String, force_fsync: bool) -> FileStorage {
        FileStorage {
            data_dir,
            dir_struct,
            force_fsync,
        }
    }

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

    async fn get_contents(
        &self,
        file_info: &FileInfo,
        _request: &axum::extract::Request,
    ) -> RustusResult<Response> {
        if file_info.path.is_none() {
            return Err(RustusError::FileNotFound.into());
        };
        let file = tokio::fs::File::open(file_info.path.clone().unwrap().as_str())
            .await
            .unwrap();
        let buf_file = tokio::io::BufReader::new(file);
        let reader = tokio_util::io::ReaderStream::new(buf_file);
        let body = axum::body::Body::from_stream(reader);
        let disp = generate_disposition(file_info.get_filename());
        let headers = [disp.0, disp.1];
        Ok((headers, body).into_response())
    }

    async fn add_bytes(&self, file_info: &FileInfo, bytes: Bytes) -> RustusResult<()> {
        // In normal situation this `if` statement is not
        // gonna be called, but what if it is ...
        if file_info.path.is_none() {
            return Err(RustusError::FileNotFound.into());
        }
        let path = file_info.path.as_ref().unwrap().clone();
        let force_sync = self.force_fsync;
        tokio::task::spawn_blocking(move || {
            // Opening file in w+a mode.
            // It means that we're going to append some
            // bytes to the end of a file.
            let file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(false)
                .read(false)
                .truncate(false)
                .open(path.as_str())?;
            let mut writer = BufWriter::new(file);
            writer.write_all(bytes.as_ref())?;
            writer.flush()?;
            if force_sync {
                writer.get_ref().sync_data()?;
            }
            Ok(())
        })
        .await?
    }

    async fn create_file(&self, file_info: &FileInfo) -> RustusResult<String> {
        // New path to file.
        let file_path = self.data_file_path(file_info.id.as_str())?;
        tokio::task::spawn_blocking(move || {
            // Creating new file.
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .create_new(true)
                .open(file_path.as_path())?;
            Ok(file_path.display().to_string())
        })
        .await?
    }

    async fn concat_files(
        &self,
        file_info: &FileInfo,
        parts_info: Vec<FileInfo>,
    ) -> RustusResult<()> {
        let force_fsync = self.force_fsync;
        let path = file_info.path.as_ref().unwrap().clone();
        tokio::task::spawn_blocking(move || {
            let file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(path)?;
            let mut writer = BufWriter::new(file);
            for part in parts_info {
                if part.path.is_none() {
                    return Err(RustusError::FileNotFound.into());
                }
                let part_file = OpenOptions::new()
                    .read(true)
                    .open(part.path.as_ref().unwrap())?;
                let mut reader = BufReader::new(part_file);
                copy(&mut reader, &mut writer)?;
            }
            writer.flush()?;
            if force_fsync {
                writer.get_ref().sync_data()?;
            }
            Ok(())
        })
        .await?
    }

    async fn remove_file(&self, file_info: &FileInfo) -> RustusResult<()> {
        let info = file_info.clone();
        tokio::task::spawn_blocking(move || {
            // Let's remove the file itself.
            let data_path = PathBuf::from(info.path.as_ref().unwrap().clone());
            if !data_path.exists() {
                return Err(RustusError::FileNotFound.into());
            }
            remove_file(data_path).map_err(|err| {
                error!("{:?}", err);
                RustusError::UnableToRemove(info.id.clone())
            })?;
            Ok(())
        })
        .await?
    }
}
