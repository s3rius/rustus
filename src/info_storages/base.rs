use crate::{errors::RustusResult, models::file_info::FileInfo};
/// Trait for every info storage.
///
/// This trait defines required functions
/// for building your own info storage.
pub trait InfoStorage {
    /// Prepare storage for storing files.
    ///
    /// In this function you can prepare
    /// you info storage. E.G. create a table in a database,
    /// or a directory somewhere.
    async fn prepare(&mut self) -> RustusResult<()>;

    /// Set information about an upload.
    ///
    /// This function **must** persist information
    /// about given upload so it can be accessed again by file_id.
    ///
    /// The `create` parameter is for optimizations.
    /// It's here, because some storages like databases have to
    /// be queried twice in order to get the information
    /// about a file and actually store it. To bypass it
    /// we can guarantee that this parameter will never be `true`
    /// for any update operation.
    async fn set_info(&self, file_info: &FileInfo, create: bool) -> RustusResult<()>;

    /// Retrieve information from storage.
    ///
    /// This function must return information about file
    /// from the given storage.
    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo>;

    /// This function removes information about file completely.
    ///
    /// This function must actually delete any stored information
    /// associated with the given `file_id`.
    async fn remove_info(&self, file_id: &str) -> RustusResult<()>;
}
