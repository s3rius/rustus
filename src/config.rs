use std::path::PathBuf;

use structopt::StructOpt;

use crate::errors::TuserError;
use crate::storages::AvailableStores;

#[derive(StructOpt, Debug, Clone)]
pub struct StorageOptions {
    /// Tuser storage type.
    #[structopt(long, short, default_value = "file_storage", env = "TUSER_STORAGE")]
    pub storage: AvailableStores,

    /// Tuser data directory
    ///
    /// This directory is used to store files
    /// for all *file_storage storages.
    #[structopt(
        long,
        default_value = "./data",
        required_if("storage", "file_storage"),
        required_if("storage", "sqlite_file_storage")
    )]
    pub data: PathBuf,

    /// Path to SQLite file.
    ///
    /// This file is used to
    /// store information about uploaded files.
    #[structopt(
        long,
        default_value = "data/info.sqlite3",
        required_if("storage", "sqlite_file_storage")
    )]
    pub sqlite_dsn: PathBuf,
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "tuser", about = "Tus server implementation in Rust.")]
pub struct TuserConf {
    /// Tuser host
    #[structopt(short, long, default_value = "0.0.0.0", env = "TUSER_HOST")]
    pub host: String,

    /// Tuser server port
    #[structopt(short, long, default_value = "1081", env = "TUSER_PORT")]
    pub port: u16,

    /// Tuser base API url
    #[structopt(long, default_value = "/files", env = "TUSER_URL")]
    pub url: String,

    /// Enabled hooks for http events
    #[structopt(long, default_value = "pre-create,post-finish", env = "TUSER_HOOKS")]
    pub enabled_hooks: String,

    /// Tuser maximum log level
    #[structopt(long, default_value = "INFO", env = "TUSER_LOG_LEVEL")]
    pub log_level: log::LevelFilter,

    /// Number of actix workers default value = number of cpu cores.
    #[structopt(long, short, env = "TUSER_WORKERS")]
    pub workers: Option<usize>,

    /// Enabled extensions for TUS protocol.
    #[structopt(
        long,
        default_value = "creation,creation-with-upload,getting",
        env = "TUSER_EXTENSIONS"
    )]
    pub extensions: String,

    #[structopt(flatten)]
    pub storage_opts: StorageOptions,
}

/// Enum of available Protocol Extensions
#[derive(PartialEq, PartialOrd, Ord, Eq)]
pub enum ProtocolExtensions {
    CreationWithUpload,
    Creation,
    Termination,
    Getting,
}

impl TryFrom<String> for ProtocolExtensions {
    type Error = TuserError;

    /// Parse string to protocol extension.
    ///
    /// This function raises an error if unknown protocol was passed.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "creation" => Ok(ProtocolExtensions::Creation),
            "creation-with-upload" => Ok(ProtocolExtensions::CreationWithUpload),
            "termination" => Ok(ProtocolExtensions::Termination),
            "getting" => Ok(ProtocolExtensions::Getting),
            _ => Err(TuserError::UnknownExtension(value.clone())),
        }
    }
}

impl From<ProtocolExtensions> for String {
    /// Mapping protocol extensions to their
    /// original names.
    fn from(ext: ProtocolExtensions) -> Self {
        match ext {
            ProtocolExtensions::Creation => "creation".into(),
            ProtocolExtensions::CreationWithUpload => "creation-with-upload".into(),
            ProtocolExtensions::Termination => "termination".into(),
            ProtocolExtensions::Getting => "getting".into(),
        }
    }
}

impl TuserConf {
    /// Function to parse CLI parametes.
    ///
    /// This is a workaround for issue mentioned
    /// [here](https://www.reddit.com/r/rust/comments/8ddd19/confusion_with_splitting_mainrs_into_smaller/).
    pub fn from_args() -> TuserConf {
        <TuserConf as StructOpt>::from_args()
    }

    /// Base API url.
    pub fn base_url(&self) -> String {
        format!(
            "/{}",
            self.url
                .strip_prefix('/')
                .unwrap_or_else(|| self.url.as_str())
        )
    }

    /// URL for a particular file.
    pub fn file_url(&self) -> String {
        let base_url = self.base_url();
        format!(
            "{}/{{file_id}}",
            base_url
                .strip_suffix('/')
                .unwrap_or_else(|| base_url.as_str())
        )
    }

    /// List of extensions.
    ///
    /// This function will parse list of extensions from CLI
    /// and sort them.
    ///
    /// Protocol extensions must be sorted,
    /// because Actix doesn't override
    /// existing methods.
    pub fn extensions_vec(&self) -> Vec<ProtocolExtensions> {
        let mut ext = self
            .extensions
            .split(',')
            .flat_map(|ext| ProtocolExtensions::try_from(String::from(ext)))
            .collect::<Vec<ProtocolExtensions>>();
        // creation-with-upload
        if ext.contains(&ProtocolExtensions::CreationWithUpload)
            && !ext.contains(&ProtocolExtensions::Creation)
        {
            ext.push(ProtocolExtensions::Creation);
        }
        ext.sort();
        ext
    }
}
