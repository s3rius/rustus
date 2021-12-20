use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use chrono::{Datelike, Timelike};
use structopt::StructOpt;

use crate::errors::RustusError;
use crate::info_storages::AvailableInfoStores;
use crate::storages::AvailableStores;

#[derive(StructOpt, Debug, Clone)]
pub struct StorageOptions {
    /// Rustus storage type.
    ///
    /// Storages are used to store
    /// uploads.
    #[structopt(long, short, default_value = "file_storage", env = "RUSTUS_STORAGE")]
    pub storage: AvailableStores,

    /// Rustus data directory
    ///
    /// This directory is used to store files
    /// for all *file_storage storages.
    #[structopt(long, default_value = "./data")]
    pub data_dir: PathBuf,

    #[structopt(long, short = "dstruct", default_value = "")]
    pub dis_structure: String,
}

#[derive(StructOpt, Debug, Clone)]
pub struct InfoStoreOptions {
    /// Type of info storage.
    ///
    /// Info storages are used
    /// to store information about
    /// uploads.
    ///
    /// This information is used in
    /// HEAD requests.
    #[structopt(
        long,
        short,
        default_value = "file_info_storage",
        env = "RUSTUS_INFO_STORAGE"
    )]
    pub info_storage: AvailableInfoStores,

    /// Rustus info directory
    ///
    /// This directory is used to store .info files
    /// for `file_info_storage`.
    #[structopt(long, default_value = "./data", env = "RUSTUS_INFO_DIR")]
    pub info_dir: PathBuf,

    #[structopt(
        long,
        required_if("info-storage", "db_info_storage"),
        required_if("info-storage", "redis_info_storage"),
        env = "RUSTUS_INFO_DB_DSN"
    )]
    pub info_db_dsn: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "rustus")]
/// Tus protocol implementation.
///
/// This program is a web-server that
/// implements protocol for resumable uploads.
///
/// You can read more about protocol
/// [here](https://tus.io/).
pub struct RustusConf {
    /// Rustus host
    #[structopt(short, long, default_value = "0.0.0.0", env = "RUSTUS_HOST")]
    pub host: String,

    /// Rustus server port
    #[structopt(short, long, default_value = "1081", env = "RUSTUS_PORT")]
    pub port: u16,

    /// Rustus base API url
    #[structopt(long, default_value = "/files", env = "RUSTUS_URL")]
    pub url: String,

    #[structopt(
        long,
        short = "mbs",
        default_value = "262144",
        env = "RUSTUS_MAX_BODY_SIZE"
    )]
    pub max_body_size: usize,

    /// Enabled hooks for http events
    #[structopt(long, default_value = "pre-create,post-finish", env = "RUSTUS_HOOKS")]
    pub enabled_hooks: String,

    /// Rustus maximum log level
    #[structopt(long, default_value = "INFO", env = "RUSTUS_LOG_LEVEL")]
    pub log_level: log::LevelFilter,

    /// Number of actix workers default value = number of cpu cores.
    #[structopt(long, short, env = "RUSTUS_WORKERS")]
    pub workers: Option<usize>,

    /// Enabled extensions for TUS protocol.
    #[structopt(
        long,
        default_value = "getting,creation,termination,creation-with-upload,creation-defer-length",
        env = "RUSTUS_EXTENSIONS"
    )]
    pub extensions: String,

    #[structopt(flatten)]
    pub storage_opts: StorageOptions,

    #[structopt(flatten)]
    pub info_storage_opts: InfoStoreOptions,
}

/// Enum of available Protocol Extensions
#[derive(PartialEq, PartialOrd, Ord, Eq)]
pub enum ProtocolExtensions {
    CreationDeferLength,
    CreationWithUpload,
    Creation,
    Termination,
    Getting,
}

impl TryFrom<String> for ProtocolExtensions {
    type Error = RustusError;

    /// Parse string to protocol extension.
    ///
    /// This function raises an error if unknown protocol was passed.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "creation" => Ok(ProtocolExtensions::Creation),
            "creation-with-upload" => Ok(ProtocolExtensions::CreationWithUpload),
            "creation-defer-length" => Ok(ProtocolExtensions::CreationDeferLength),
            "termination" => Ok(ProtocolExtensions::Termination),
            "getting" => Ok(ProtocolExtensions::Getting),
            _ => Err(RustusError::UnknownExtension(value.clone())),
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
            ProtocolExtensions::CreationDeferLength => "creation-defer-length".into(),
        }
    }
}

impl RustusConf {
    /// Function to parse CLI parametes.
    ///
    /// This is a workaround for issue mentioned
    /// [here](https://www.reddit.com/r/rust/comments/8ddd19/confusion_with_splitting_mainrs_into_smaller/).
    pub fn from_args() -> RustusConf {
        <RustusConf as StructOpt>::from_args()
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

    pub fn dir_struct(&self) -> String {
        let now = chrono::Utc::now();
        let mut vars: HashMap<String, String> = HashMap::new();
        vars.insert("day".into(), now.day().to_string());
        vars.insert("month".into(), now.month().to_string());
        vars.insert("year".into(), now.year().to_string());
        vars.insert("hour".into(), now.hour().to_string());
        vars.insert("minute".into(), now.minute().to_string());
        for (key, value) in env::vars() {
            vars.insert(format!("env[{}]", key), value);
        }
        strfmt::strfmt(self.storage_opts.dis_structure.as_str(), &vars)
            .unwrap_or_else(|_| "".into())
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

        // If create-with-upload extension is enabled
        // creation extension must be enabled too.
        if ext.contains(&ProtocolExtensions::CreationWithUpload)
            && !ext.contains(&ProtocolExtensions::Creation)
        {
            ext.push(ProtocolExtensions::Creation);
        }

        // If create-defer-length extension is enabled
        // creation extension must be enabled too.
        if ext.contains(&ProtocolExtensions::CreationDeferLength)
            && !ext.contains(&ProtocolExtensions::Creation)
        {
            ext.push(ProtocolExtensions::Creation);
        }

        ext.sort();
        ext
    }
}
