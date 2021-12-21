use std::collections::HashMap;

use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Information about file.
/// It has everything about stored file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: String,
    pub offset: usize,
    pub length: usize,
    pub path: String,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
    pub deferred_size: bool,
    pub metadata: HashMap<String, String>,
}

impl FileInfo {
    /// Creates new `FileInfo`.
    ///
    /// # Params
    ///
    /// File info takes
    /// `file_id` - Unique file identifier;
    /// `file_size` - Size of a file if it's known;
    /// `path` - local path of a file;
    /// `initial_metadata` - meta information, that could be omitted.
    pub fn new(
        file_id: &str,
        file_size: Option<usize>,
        path: String,
        initial_metadata: Option<HashMap<String, String>>,
    ) -> FileInfo {
        let id = String::from(file_id);
        let mut length = 0;
        let mut deferred_size = true;
        if let Some(size) = file_size {
            length = size;
            deferred_size = false;
        }
        let metadata = match initial_metadata {
            Some(meta) => meta,
            None => HashMap::new(),
        };

        FileInfo {
            id,
            path,
            length,
            metadata,
            deferred_size,
            offset: 0,
            created_at: chrono::Utc::now(),
        }
    }
}
