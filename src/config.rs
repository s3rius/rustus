use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use chrono::{Datelike, Timelike};
use lazy_static::lazy_static;
use log::error;
use structopt::StructOpt;

use crate::info_storages::AvailableInfoStores;
use crate::notifiers::{Format, Hook};
use crate::protocol::extensions::Extensions;

use crate::storages::AvailableStores;

lazy_static! {
    /// Freezing ENVS on startup.
    static ref ENV_MAP: HashMap<String, String> = {
        let mut m = HashMap::new();
        for (key, value) in env::vars() {
            m.insert(format!("env[{}]", key), value);
        }
        m
    };
}

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

    #[cfg(feature = "file_notifiers")]
    #[structopt(long, env = "RUSTUS_HOOKS_DIR")]
    pub hooks_dir: Option<PathBuf>,

    #[cfg(feature = "file_notifiers")]
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
    #[structopt(short, long, default_value = "0.0.0.0", env = "RUSTUS_HOST")]
    pub host: String,

    /// Rustus server port
    #[structopt(short, long, default_value = "1081", env = "RUSTUS_PORT")]
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
        default_value = "getting,creation,termination,creation-with-upload,creation-defer-length",
        env = "RUSTUS_TUS_EXTENSIONS",
        use_delimiter = true
    )]
    pub tus_extensions: Vec<Extensions>,

    #[structopt(flatten)]
    pub storage_opts: StorageOptions,

    #[structopt(flatten)]
    pub info_storage_opts: InfoStoreOptions,

    #[structopt(flatten)]
    pub notification_opts: NotificationsOptions,
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

    /// Check if hook is enabled by user.
    pub fn hook_is_active(&self, hook: Hook) -> bool {
        self.notification_opts.hooks.contains(&hook)
    }

    /// Generate directory name with user template.
    pub fn dir_struct(&self) -> String {
        let now = chrono::Utc::now();
        let mut vars: HashMap<String, String> = ENV_MAP.clone();
        vars.insert("day".into(), now.day().to_string());
        vars.insert("month".into(), now.month().to_string());
        vars.insert("year".into(), now.year().to_string());
        vars.insert("hour".into(), now.hour().to_string());
        vars.insert("minute".into(), now.minute().to_string());
        strfmt::strfmt(self.storage_opts.dir_structure.as_str(), &vars).unwrap_or_else(|err| {
            error!("{}", err);
            "".into()
        })
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
