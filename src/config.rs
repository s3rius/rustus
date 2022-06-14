use std::{ffi::OsString, path::PathBuf};

use structopt::StructOpt;

use crate::{
    info_storages::AvailableInfoStores,
    notifiers::{Format, Hook},
    protocol::extensions::Extensions,
};

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

    /// Storage directory structure.
    /// This template shows inner directory structure.
    /// You can use following variables:
    /// day, month, year or even environment variables.
    /// Example: "/year/month/day/env[HOSTNAME]/".
    ///
    #[structopt(long, env = "RUSTUS_DIR_STRUCTURE", default_value = "")]
    pub dir_structure: String,

    /// Forces fsync call after writing chunk to filesystem.
    /// This parameter can help you when working with
    /// Network file systems. It guarantees that
    /// everything is written on disk correctly.
    ///
    /// In most cases this parameter is redundant.
    #[structopt(long, env = "RUSTUS_FORCE_FSYNC")]
    pub force_fsync: bool,
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
#[allow(clippy::struct_excessive_bools)]
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

    /// Rustus will create exchange if enabled.
    #[cfg(feature = "amqp_notifier")]
    #[structopt(long, env = "RUSTUS_HOOKS_AMQP_DECLARE_EXCHANGE")]
    pub hooks_amqp_declare_exchange: bool,

    /// Rustus will create all queues for communication and bind them
    /// to exchange if enabled.
    #[cfg(feature = "amqp_notifier")]
    #[structopt(long, env = "RUSTUS_HOOKS_AMQP_DECLARE_QUEUES")]
    pub hooks_amqp_declare_queues: bool,

    /// Durability type of exchange.
    #[cfg(feature = "amqp_notifier")]
    #[structopt(long, env = "RUSTUS_HOOKS_AMQP_DURABLE_EXCHANGE")]
    pub hooks_amqp_durable_exchange: bool,

    /// Durability type of queues.
    #[cfg(feature = "amqp_notifier")]
    #[structopt(long, env = "RUSTUS_HOOKS_AMQP_DURABLE_QUEUES")]
    pub hooks_amqp_durable_queues: bool,

    /// Adds celery specific headers.
    #[cfg(feature = "amqp_notifier")]
    #[structopt(long, env = "RUSTUS_HOOKS_AMQP_CELERY")]
    pub hooks_amqp_celery: bool,

    /// Name of amqp exchange.
    #[cfg(feature = "amqp_notifier")]
    #[structopt(long, env = "RUSTUS_HOOKS_AMQP_EXCHANGE", default_value = "rustus")]
    pub hooks_amqp_exchange: String,

    /// Exchange kind.
    #[cfg(feature = "amqp_notifier")]
    #[structopt(long, env = "RUSTUS_HOOKS_AMQP_EXCHANGE_KIND", default_value = "topic")]
    pub hooks_amqp_exchange_kind: String,

    /// Routing key to use when sending message to an exchange.
    #[cfg(feature = "amqp_notifier")]
    #[structopt(long, env = "RUSTUS_HOOKS_AMQP_ROUTING_KEY")]
    pub hooks_amqp_routing_key: Option<String>,

    /// Prefix for all AMQP queues.
    #[cfg(feature = "amqp_notifier")]
    #[structopt(
        long,
        env = "RUSTUS_HOOKS_AMQP_QUEUES_PREFIX",
        default_value = "rustus"
    )]
    pub hooks_amqp_queues_prefix: String,

    /// Directory for executable hook files.
    /// This parameter is used to call executables from dir.
    #[structopt(long, env = "RUSTUS_HOOKS_DIR")]
    pub hooks_dir: Option<PathBuf>,

    /// Executable file which must be called for
    /// notifying about upload status.
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

    /// Allowed hosts for CORS protocol.
    ///
    /// By default all hosts are allowed.
    #[structopt(long, env = "RUSTUS_CORS", use_delimiter = true)]
    pub cors: Vec<String>,

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
        default_value = "getting,creation,termination,creation-with-upload,creation-defer-length,concatenation,checksum",
        env = "RUSTUS_TUS_EXTENSIONS",
        use_delimiter = true
    )]
    pub tus_extensions: Vec<Extensions>,

    /// Remove part files after concatenation is done.
    /// By default rustus does nothing with part files after concatenation.
    ///
    /// This parameter is only needed if concatenation extension is enabled.
    #[structopt(long, env = "RUSTUS_REMOVE_PARTS")]
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
            self.url.strip_prefix('/').unwrap_or(self.url.as_str())
        )
    }

    /// Helper for generating URI for test files.
    #[cfg(test)]
    pub fn file_url(&self, file_id: &str) -> String {
        let base_url = self.base_url();
        format!(
            "{}/{}",
            base_url.strip_suffix('/').unwrap_or(base_url.as_str()),
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
