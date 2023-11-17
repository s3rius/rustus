use std::path::PathBuf;

use clap::Parser;

use crate::{
    data_storage::AvailableStorages, extensions::TusExtensions,
    info_storages::AvailableInfoStorages,
};

#[derive(Parser, Clone, Debug)]
pub struct InfoStorageConfig {
    #[arg(long, default_value = "file", env = "RUSTUS_INFO_STORAGE")]
    pub info_storage: AvailableInfoStorages,

    /// Rustus info directory
    ///
    /// This directory is used to store .info files
    /// for `file_info_storage`.
    #[arg(long, default_value = "./data", env = "RUSTUS_INFO_DIR")]
    pub info_dir: PathBuf,

    /// Connection string for remote info storages.
    ///
    /// This connection string is used for storages
    /// which require connection. Examples of such storages
    /// are `Postgres`, `MySQL` or `Redis`.
    ///
    /// Value must include all connection details.
    #[arg(
        long,
        required_if_eq_any([("info_storage", "redis")]),
        env = "RUSTUS_INFO_DB_DSN"
    )]
    pub info_db_dsn: Option<String>,

    #[arg(long, env = "RUSTUS_REDIS_INFO_EXPIRATION")]
    pub redis_info_expiration: Option<usize>,
}

#[derive(Parser, Clone, Debug)]
pub struct DataStorageConfig {
    /// Rustus storage type.
    ///
    /// Storages are used to store
    /// uploads.
    #[arg(long, short, default_value = "file", env = "RUSTUS_STORAGE")]
    pub data_storage: AvailableStorages,

    /// Rustus data directory
    ///
    /// This directory is used to store files
    /// for all *file_storage storages.
    #[arg(long, env = "RUSTUS_DATA_DIR", default_value = "./data")]
    pub data_dir: PathBuf,

    /// Storage directory structure.
    /// This template shows inner directory structure.
    /// You can use following variables:
    /// day, month, year or even environment variables.
    /// Example: "/year/month/day/env[HOSTNAME]/".
    ///
    #[arg(long, env = "RUSTUS_DIR_STRUCTURE", default_value = "")]
    pub dir_structure: String,

    /// Forces fsync call after writing chunk to filesystem.
    /// This parameter can help you when working with
    /// Network file systems. It guarantees that
    /// everything is written on disk correctly.
    ///
    /// In most cases this parameter is redundant.
    #[arg(long, env = "RUSTUS_FORCE_FSYNC")]
    pub force_fsync: bool,

    /// S3 bucket to upload files to.
    ///
    /// This parameter is required fo s3-based storages.
    #[arg(long, required_if_eq("data_storage", "hybrid-s3"), env = "RUSTUS_S3_BUCKET")]
    pub s3_bucket: Option<String>,

    /// S3 region.
    ///
    /// This parameter is required fo s3-based storages.
    #[arg(long, required_if_eq("data_storage", "hybrid-s3"), env = "RUSTUS_S3_REGION")]
    pub s3_region: Option<String>,

    /// S3 access key.
    ///
    /// This parameter is required fo s3-based storages.
    #[arg(long, env = "RUSTUS_S3_ACCESS_KEY")]
    pub s3_access_key: Option<String>,

    /// S3 access key path.
    ///
    /// This parameter is used fo s3-based storages.
    /// path to file that has s3-access-key inside.
    #[arg(long, env = "RUSTUS_S3_ACCESS_KEY_PATH")]
    pub s3_access_key_path: Option<PathBuf>,

    /// S3 secret key.
    ///
    /// This parameter is required fo s3-based storages.
    #[arg(long, env = "RUSTUS_S3_SECRET_KEY")]
    pub s3_secret_key: Option<String>,

    /// S3 secret key path.
    ///
    /// This parameter is required fo s3-based storages.
    /// path to file that has s3-secret-key inside.
    #[arg(long, env = "RUSTUS_S3_SECRET_KEY_PATH")]
    pub s3_secret_key_path: Option<PathBuf>,

    /// S3 URL.
    ///
    /// This parameter is required fo s3-based storages.
    #[arg(long, required_if_eq("data_storage", "hybrid-s3"), env = "RUSTUS_S3_URL")]
    pub s3_url: Option<String>,

    /// S3 force path style.
    ///
    /// This parameter is required fo s3-based storages.
    #[arg(long, env = "RUSTUS_S3_FORCE_PATH_STYLE")]
    pub s3_force_path_style: bool,

    /// S3 security token.
    ///
    /// This parameter is required fo s3-based storages.
    #[arg(long, env = "RUSTUS_S3_SECURITY_TOKEN")]
    pub s3_security_token: Option<String>,

    /// S3 session token.
    ///
    /// This parameter is required fo s3-based storages.
    #[arg(long, env = "RUSTUS_S3_SESSION_TOKEN")]
    pub s3_session_token: Option<String>,

    /// S3 profile.
    ///
    /// This parameter is required fo s3-based storages.
    #[arg(long, env = "RUSTUS_S3_PROFILE")]
    pub s3_profile: Option<String>,

    /// Additional S3 headers.
    /// These headers are passed to every request to s3.
    /// Useful for configuring ACLs.
    ///
    /// This parameter is required fo s3-based storages.
    #[arg(long, env = "RUSTUS_S3_HEADERS")]
    pub s3_headers: Option<String>,
}

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    #[arg(long, default_value = "0.0.0.0", env = "RUSTUS_SERVER_HOST")]
    pub host: String,
    #[arg(long, default_value = "1081", env = "RUSTUS_SERVER_PORT")]
    pub port: u16,
    #[arg(long, default_value = "INFO", env = "RUSTUS_LOG_LEVEL")]
    pub log_level: log::LevelFilter,

    #[arg(long, default_value = "/files", env = "RUSTUS_PREFIX")]
    pub url: String,

    /// Enabling this parameter
    /// Will allow creation of empty files
    /// when Upload-Length header equals to 0.
    #[arg(long, env = "RUSTUS_ALLOW_EMPTY")]
    pub allow_empty: bool,

    /// Remove part files after concatenation is done.
    /// By default rustus does nothing with part files after concatenation.
    ///
    /// This parameter is only needed if concatenation extension is enabled.
    #[arg(long, env = "RUSTUS_REMOVE_PARTS")]
    pub remove_parts: bool,

    /// Maximum payload size in bytes.
    ///
    /// This limit used to reduce amount of consumed memory.
    #[arg(
        long,
        short = 'm',
        default_value = "262144",
        env = "RUSTUS_MAX_BODY_SIZE"
    )]
    pub max_body_size: usize,

    /// Maximum size of file that can be uploaded.
    ///
    /// If not set, file size is unlimited.
    #[arg(long, env = "RUSTUS_MAX_FILE_SIZE")]
    pub max_file_size: Option<usize>,

    #[arg(
        long,
        default_value = "getting,creation,termination,creation-with-upload,creation-defer-length,concatenation,checksum",
        env = "RUSTUS_TUS_EXTENSIONS",
        use_value_delimiter = true
    )]
    pub tus_extensions: Vec<TusExtensions>,

    // We skip this argument, because we won't going to
    // fullfill it from CLI. This argument is populated based
    // on `tus_extensions` argument.
    #[arg(skip)]
    pub tus_extensions_set: rustc_hash::FxHashSet<TusExtensions>,

    #[command(flatten)]
    pub info_storage_config: InfoStorageConfig,

    #[command(flatten)]
    pub data_storage_config: DataStorageConfig,
}

impl Config {
    pub fn parse() -> Self {
        let mut config = <Self as Parser>::parse();
        config.prepare();
        config
    }

    pub fn prepare(&mut self) {
        // Update URL prefix. This is needed to make sure that
        // URLs are correctly generated.
        self.url = self.url.trim_end_matches('/').to_string();

        // We want to build a hashmap with all extensions. Because it
        // is going to be much faster to work with in the future.
        for extension in self.tus_extensions.clone() {
            if extension == TusExtensions::CreationWithUpload
                || extension == TusExtensions::CreationDeferLength
            {
                self.tus_extensions_set.insert(TusExtensions::Creation);
            }
            self.tus_extensions_set.insert(extension);
        }
    }

    pub fn get_url(&self, url: &str) -> String {
        format!("{}/{url}", self.url)
    }
}
