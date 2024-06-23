use base64::{engine::general_purpose, Engine};
use chrono::{serde::ts_seconds, DateTime, Utc};
use rustc_hash::FxHashMap;
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
    pub metadata: FxHashMap<String, String>,
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
    #[must_use]
    pub fn new(
        file_id: &str,
        length: Option<usize>,
        path: Option<String>,
        storage: String,
        initial_metadata: Option<FxHashMap<String, String>>,
    ) -> Self {
        let id = String::from(file_id);

        let deferred_size = length.is_none();
        let metadata = initial_metadata.unwrap_or_default();

        Self {
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
    #[must_use]
    pub fn get_metadata_string(&self) -> Option<String> {
        let mut result = Vec::new();

        // Getting all metadata keys.
        for (key, val) in &self.metadata {
            let encoded_value = general_purpose::STANDARD.encode(val);
            // Adding metadata entry to the list.
            result.push(format!("{key} {encoded_value}"));
        }

        if result.is_empty() {
            None
        } else {
            // Merging the metadata.
            Some(result.join(","))
        }
    }

    #[must_use]
    pub fn get_filename(&self) -> &str {
        self.metadata
            .get("filename")
            .or_else(|| self.metadata.get("name"))
            .unwrap_or(&self.id)
    }

    #[cfg(test)]
    #[must_use]
    pub fn new_test() -> Self {
        Self::new(
            uuid::Uuid::new_v4().to_string().as_str(),
            Some(10),
            Some("random_path".into()),
            "random_storage".into(),
            None,
        )
    }
}
