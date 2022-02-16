use std::ffi::OsString;
use std::path::PathBuf;

use structopt::StructOpt;

use crate::info_storages::AvailableInfoStores;
use crate::notifiers::{Format, Hook};
use crate::protocol::extensions::Extensions;

use crate::storages::AvailableStores;

#[derive(StructOpt, Debug, Clone)]
pub struct StorageOptions {
    /// Rustus storage type.
    ///
    /// Storages are used to store
    /// uploads.
    #[structopt(long, short, default_value = "file-storage", env = "RUSTUS_STORAGE")]
    pub storage: AvailableStores,

    /// Rustus data directory
    ///
    /// This directory is used to store files
    /// for all *file_storage storages.
    #[structopt(long, env = "RUSTUS_DATA_DIR", default_value = "./data")]
    pub data_dir: PathBuf,

    #[structopt(long, env = "RUSTUS_DIR_STRUCTURE", default_value = "")]
    pub dir_structure: String,
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
        default_value = "file-info-storage",
        env = "RUSTUS_INFO_STORAGE"
    )]
    pub info_storage: AvailableInfoStores,

    /// Rustus info directory
    ///
    /// This directory is used to store .info files
    /// for `file_info_storage`.
    #[structopt(long, default_value = "./data", env = "RUSTUS_INFO_DIR")]
    pub info_dir: PathBuf,

    /// Connection string for remote info storages.
    ///
    /// This connection string is used for storages
    /// which require connection. Examples of such storages
    /// are `Postgres`, `MySQL` or `Redis`.
    ///
    /// Value must include all connection details.
    #[cfg(any(feature = "redis_info_storage", feature = "db_info_storage"))]
    #[structopt(
        long,
        required_if("info-storage", "db-info-storage"),
        required_if("info-storage", "redis-info-storage"),
        env = "RUSTUS_INFO_DB_DSN"
    )]
    pub info_db_dsn: Option<String>,
}

#[derive(StructOpt, Debug, Clone)]
pub struct NotificationsOptions {
    /// Notifications format.
    ///
    /// This format will be used in all
    /// messages about hooks.
    #[structopt(long, default_value = "default", env = "RUSTUS_HOOKS_FORMAT")]
    pub hooks_format: Format,

    /// Enabled hooks for notifications.
    #[structopt(
        long,
        default_value = "pre-create,post-create,post-receive,post-terminate,post-finish",
        env = "RUSTUS_HOOKS",
        use_delimiter = true
    )]
    pub hooks: Vec<Hook>,

    /// List of URLS to send webhooks to.
    #[cfg(feature = "http_notifier")]
    #[structopt(long, env = "RUSTUS_HOOKS_HTTP_URLS", use_delimiter = true)]
    pub hooks_http_urls: Vec<String>,

    // List of headers to forward from client.
    #[cfg(feature = "http_notifier")]
    #[structopt(long, env = "RUSTUS_HOOKS_HTTP_PROXY_HEADERS", use_delimiter = true)]
    pub hooks_http_proxy_headers: Vec<String>,

    /// Url for AMQP server.
    #[cfg(feature = "amqp_notifier")]
    #[structopt(long, env = "RUSTUS_HOOKS_AMQP_URL")]
    pub hooks_amqp_url: Option<String>,

    /// Name of amqp exchange.
    #[cfg(feature = "amqp_notifier")]
    #[structopt(long, env = "RUSTUS_HOOKS_AMQP_EXCHANGE", default_value = "rustus")]
    pub hooks_amqp_exchange: String,

    #[cfg(feature = "amqp_notifier")]
    #[structopt(
        long,
        env = "RUSTUS_HOOKS_AMQP_QUEUES_PREFIX",
        default_value = "rustus"
    )]
    pub hooks_amqp_queues_prefix: String,

    #[structopt(long, env = "RUSTUS_HOOKS_DIR")]
    pub hooks_dir: Option<PathBuf>,

    #[structopt(long, env = "RUSTUS_HOOKS_FILE")]
    pub hooks_file: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "Rustus")]
/// Tus protocol implementation.
///
/// This program is a web-server that
/// implements protocol for resumable uploads.
///
/// You can read more about protocol
/// [here](https://tus.io/).
pub struct RustusConf {
    /// Rustus server host
    #[structopt(short, long, default_value = "0.0.0.0", env = "RUSTUS_SERVER_HOST")]
    pub host: String,

    /// Rustus server port
    #[structopt(short, long, default_value = "1081", env = "RUSTUS_SERVER_PORT")]
    pub port: u16,

    /// Rustus base API url
    #[structopt(long, default_value = "/files", env = "RUSTUS_URL")]
    pub url: String,

    /// Maximum payload size.
    ///
    /// This limit used to reduce amount of consumed memory.
    #[structopt(
        long,
        short = "mbs",
        default_value = "262144",
        env = "RUSTUS_MAX_BODY_SIZE"
    )]
    pub max_body_size: usize,

    /// Rustus maximum log level
    #[structopt(long, default_value = "INFO", env = "RUSTUS_LOG_LEVEL")]
    pub log_level: log::LevelFilter,

    /// Number of actix workers default value = number of cpu cores.
    #[structopt(long, short, env = "RUSTUS_WORKERS")]
    pub workers: Option<usize>,

    /// Enabled extensions for TUS protocol.
    #[structopt(
        long,
        default_value = "getting,creation,termination,creation-with-upload,creation-defer-length,concatenation",
        env = "RUSTUS_TUS_EXTENSIONS",
        use_delimiter = true
    )]
    pub tus_extensions: Vec<Extensions>,

    /// Remove part files after concatenation is done.
    /// By default rustus does nothing with part files after concatenation.
    ///
    /// This parameter is only needed if concatenation extension is enabled.
    #[structopt(long, parse(from_flag))]
    pub remove_parts: bool,

    #[structopt(flatten)]
    pub storage_opts: StorageOptions,

    #[structopt(flatten)]
    pub info_storage_opts: InfoStoreOptions,

    #[structopt(flatten)]
    pub notification_opts: NotificationsOptions,
}

#[cfg_attr(coverage, no_coverage)]
impl RustusConf {
    /// Function to parse CLI parametes.
    ///
    /// This is a workaround for issue mentioned
    /// [here](https://www.reddit.com/r/rust/comments/8ddd19/confusion_with_splitting_mainrs_into_smaller/).
    pub fn from_args() -> RustusConf {
        <RustusConf as StructOpt>::from_args()
    }

    pub fn from_iter<I>(iter: I) -> RustusConf
    where
        I: IntoIterator,
        I::Item: Into<OsString> + Clone,
    {
        <RustusConf as StructOpt>::from_iter(iter)
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

    /// Helper for generating URI for test files.
    #[cfg(test)]
    pub fn file_url(&self, file_id: &str) -> String {
        let base_url = self.base_url();
        format!(
            "{}/{}",
            base_url
                .strip_suffix('/')
                .unwrap_or_else(|| base_url.as_str()),
            file_id
        )
    }

    /// Check if hook is enabled by user.
    pub fn hook_is_active(&self, hook: Hook) -> bool {
        self.notification_opts.hooks.contains(&hook)
    }

    /// List of extensions.
    ///
    /// This function will parse list of extensions from CLI
    /// and sort them.
    ///
    /// Protocol extensions must be sorted,
    /// because Actix doesn't override
    /// existing methods.
    pub fn extensions_vec(&self) -> Vec<Extensions> {
        let mut ext = self.tus_extensions.clone();

        // If create-with-upload extension is enabled
        // creation extension must be enabled too.
        if ext.contains(&Extensions::CreationWithUpload) && !ext.contains(&Extensions::Creation) {
            ext.push(Extensions::Creation);
        }

        // If create-defer-length extension is enabled
        // creation extension must be enabled too.
        if ext.contains(&Extensions::CreationDeferLength) && !ext.contains(&Extensions::Creation) {
            ext.push(Extensions::Creation);
        }

        ext.sort();
        ext
    }
}
