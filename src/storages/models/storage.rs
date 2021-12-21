use crate::errors::RustusResult;
use crate::info_storages::FileInfo;
use actix_files::NamedFile;
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait Storage {
    /// Prepare storage before starting up server.
    ///
    /// Function to check if configuration is correct
    /// and prepare storage E.G. create connection pool,
    /// or directory for files.
    async fn prepare(&mut self) -> RustusResult<()>;

    /// Get file information.
    ///
    /// This method returns all information about file.
    ///
    /// # Params
    /// `file_id` - unique file identifier.
    async fn get_file_info(&self, file_id: &str) -> RustusResult<FileInfo>;

    /// Get contents of a file.
    ///
    /// This method must return NamedFile since it
    /// is compatible with ActixWeb files interface.
    ///
    /// # Params
    /// `file_id` - unique file identifier.
    async fn get_contents(&self, file_id: &str) -> RustusResult<NamedFile>;

    /// Add bytes to the file.
    ///
    /// This method is used to append bytes to some file.
    /// It returns new offset.
    ///
    /// # Params
    /// `file_id` - unique file identifier;
    /// `request_offset` - offset from the client.
    /// `bytes` - bytes to append to the file.
    async fn add_bytes(
        &self,
        file_id: &str,
        request_offset: usize,
        bytes: &[u8],
    ) -> RustusResult<usize>;

    /// Create file in storage.
    ///
    /// This method is used to generate unique file id, create file and store information about it.
    ///
    /// # Params
    /// `file_size` - Size of a file. It may be None if size is deferred;
    /// `metadata` - Optional file meta-information;
    async fn create_file(
        &self,
        file_size: Option<usize>,
        metadata: Option<HashMap<String, String>>,
    ) -> RustusResult<String>;

    /// Remove file from storage
    ///
    /// This method removes file and all associated
    /// object if any.
    ///
    /// # Params
    /// `file_id` - unique file identifier;
    async fn remove_file(&self, file_id: &str) -> RustusResult<()>;
}
