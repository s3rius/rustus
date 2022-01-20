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
    pub length: Option<usize>,
    pub path: Option<String>,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
    pub deferred_size: bool,
    pub is_partial: bool,
    pub is_final: bool,
    pub parts: Option<Vec<String>>,
    pub storage: String,
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
        length: Option<usize>,
        path: Option<String>,
        storage: String,
        initial_metadata: Option<HashMap<String, String>>,
    ) -> FileInfo {
        let id = String::from(file_id);

        let mut deferred_size = true;
        if length.is_some() {
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
            storage,
            metadata,
            deferred_size,
            offset: 0,
            is_final: false,
            is_partial: false,
            parts: None,
            created_at: chrono::Utc::now(),
        }
    }

    /// Function to construct `String` value
    /// from file metadata `HashMap`.
    ///
    /// This algorithm can be found at
    /// [protocol page](https://tus.io/protocols/resumable-upload.html#upload-metadata).
    pub fn get_metadata_string(&self) -> Option<String> {
        let mut result = Vec::new();

        // Getting all metadata keys.
        for (key, val) in &self.metadata {
            let encoded_value = base64::encode(val);
            // Adding metadata entry to the list.
            result.push(format!("{} {}", key, encoded_value));
        }

        if result.is_empty() {
            None
        } else {
            // Merging the metadata.
            Some(result.join(","))
        }
    }
}
