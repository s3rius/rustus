use std::{ffi::OsString, path::PathBuf};

use clap::Parser;

use crate::{
    data_storage::AvailableDataStorages,
    info_storage::AvailableInfoStorages,
    notifiers::{Format, Hook},
    protocol::extensions::Extensions,
};

#[derive(Parser, Debug, Clone)]
pub struct DataStorageOptions {
    /// Rustus storage type.
    ///
    /// Storages are used to store
    /// uploads.
    #[arg(long, short, default_value = "file-storage", env = "RUSTUS_STORAGE")]
    pub storage: AvailableDataStorages,

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
    #[arg(long, required_if_eq("storage", "hybrid-s3"), env = "RUSTUS_S3_BUCKET")]
    pub s3_bucket: Option<String>,

    /// S3 region.
    ///
    /// This parameter is required fo s3-based storages.
    #[arg(long, required_if_eq("storage", "hybrid-s3"), env = "RUSTUS_S3_REGION")]
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
    #[arg(long, required_if_eq("storage", "hybrid-s3"), env = "RUSTUS_S3_URL")]
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

#[derive(Parser, Debug, Clone)]
pub struct InfoStoreOptions {
    /// Type of info storage.
    ///
    /// Info storages are used
    /// to store information about
    /// uploads.
    ///
    /// This information is used in
    /// HEAD requests.
    #[arg(
        long,
        short,
        default_value = "file-info-storage",
        env = "RUSTUS_INFO_STORAGE"
    )]
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
        required_if_eq_any([("info_storage", "db-info-storage"), ("info_storage", "redis-info-storage")]),
        env = "RUSTUS_INFO_DB_DSN"
    )]
    pub info_db_dsn: Option<String>,

    #[arg(long, env = "RUSTUS_REDIS_INFO_EXPIRATION")]
    pub redis_info_expiration: Option<usize>,
}
#[derive(Parser, Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]

pub struct AMQPHooksOptions {
    /// Url for AMQP server.
    #[arg(long, env = "RUSTUS_HOOKS_AMQP_URL")]
    pub hooks_amqp_url: Option<String>,

    /// Rustus will create exchange if enabled.
    #[arg(long, env = "RUSTUS_HOOKS_AMQP_DECLARE_EXCHANGE")]
    pub hooks_amqp_declare_exchange: bool,

    /// Rustus will create all queues for communication and bind them
    /// to exchange if enabled.
    #[arg(long, env = "RUSTUS_HOOKS_AMQP_DECLARE_QUEUES")]
    pub hooks_amqp_declare_queues: bool,

    /// Durability type of exchange.
    #[arg(long, env = "RUSTUS_HOOKS_AMQP_DURABLE_EXCHANGE")]
    pub hooks_amqp_durable_exchange: bool,

    /// Durability type of queues.
    #[arg(long, env = "RUSTUS_HOOKS_AMQP_DURABLE_QUEUES")]
    pub hooks_amqp_durable_queues: bool,

    /// Adds celery specific headers.
    #[arg(long, env = "RUSTUS_HOOKS_AMQP_CELERY")]
    pub hooks_amqp_celery: bool,

    /// Name of amqp exchange.
    #[arg(long, env = "RUSTUS_HOOKS_AMQP_EXCHANGE", default_value = "rustus")]
    pub hooks_amqp_exchange: String,

    /// Exchange kind.
    #[arg(long, env = "RUSTUS_HOOKS_AMQP_EXCHANGE_KIND", default_value = "topic")]
    pub hooks_amqp_exchange_kind: String,

    /// Routing key to use when sending message to an exchange.
    #[arg(long, env = "RUSTUS_HOOKS_AMQP_ROUTING_KEY")]
    pub hooks_amqp_routing_key: Option<String>,

    /// Prefix for all AMQP queues.
    #[arg(
        long,
        env = "RUSTUS_HOOKS_AMQP_QUEUES_PREFIX",
        default_value = "rustus"
    )]
    pub hooks_amqp_queues_prefix: String,

    /// Maximum number of connections for RabbitMQ.
    #[arg(
        long,
        env = "RUSTUS_HOOKS_AMQP_CONNECTION_POOL_SIZE",
        default_value = "10"
    )]
    pub hooks_amqp_connection_pool_size: u64,

    /// Maximum number of opened channels for each connection.
    #[arg(
        long,
        env = "RUSTUS_HOOKS_AMQP_CHANNEL_POOL_SIZE",
        default_value = "10"
    )]
    pub hooks_amqp_channel_pool_size: u64,

    /// After this amount of time the connection will be dropped.
    #[arg(long, env = "RUSTUS_HOOKS_AMQP_IDLE_CONNECTION_TIMEOUT")]
    pub hooks_amqp_idle_connection_timeout: Option<u64>,

    /// After this amount of time in seconds, the channel will be closed.
    #[arg(long, env = "RUSTUS_HOOKS_AMQP_IDLE_CHANNELS_TIMEOUT")]
    pub hooks_amqp_idle_channels_timeout: Option<u64>,
}

#[derive(Parser, Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct NotificationsOptions {
    /// Notifications format.
    ///
    /// This format will be used in all
    /// messages about hooks.
    #[arg(long, default_value = "default", env = "RUSTUS_HOOKS_FORMAT")]
    pub hooks_format: Format,

    /// Enabled hooks for notifications.
    #[arg(
        long,
        default_value = "pre-create,post-create,post-receive,pre-terminate,post-terminate,post-finish",
        env = "RUSTUS_HOOKS",
        use_value_delimiter = true
    )]
    pub hooks: Vec<Hook>,

    /// Use this option if you use rustus
    /// behind any proxy. Like Nginx or Traefik.
    #[arg(long, env = "RUSTUS_BEHIND_PROXY")]
    pub behind_proxy: bool,

    /// List of URLS to send webhooks to.
    #[arg(long, env = "RUSTUS_HOOKS_HTTP_URLS", use_value_delimiter = true)]
    pub hooks_http_urls: Vec<String>,

    /// Timeout for all HTTP requests in seconds.
    #[arg(long, env = "RUSTUS_HTTP_HOOK_TIMEOUT")]
    pub http_hook_timeout: Option<u64>,

    // List of headers to forward from client.
    #[arg(
        long,
        env = "RUSTUS_HOOKS_HTTP_PROXY_HEADERS",
        use_value_delimiter = true
    )]
    pub hooks_http_proxy_headers: Vec<String>,

    /// Directory for executable hook files.
    /// This parameter is used to call executables from dir.
    #[arg(long, env = "RUSTUS_HOOKS_DIR")]
    pub hooks_dir: Option<PathBuf>,

    /// Executable file which must be called for
    /// notifying about upload status.
    #[arg(long, env = "RUSTUS_HOOKS_FILE")]
    pub hooks_file: Option<String>,

    #[command(flatten)]
    pub amqp_hook_opts: AMQPHooksOptions,
}

#[derive(Debug, Parser, Clone)]
pub struct SentryOptions {
    #[arg(name = "sentry-dsn", long, env = "RUSTUS_SENTRY_DSN")]
    pub dsn: Option<String>,

    #[arg(
        name = "sentry-sample-rate",
        long,
        default_value = "1.0",
        env = "RUSTUS_SENTRY_SAMPLE_RATE"
    )]
    pub sample_rate: f32,
}

#[derive(Debug, Parser, Clone)]
#[command(name = "Rustus")]
/// Tus protocol implementation.
///
/// This program is a web-server that
/// implements protocol for resumable uploads.
///
/// You can read more about protocol
/// [here](https://tus.io/).
pub struct RustusConf {
    /// Rustus server host
    #[arg(long, default_value = "0.0.0.0", env = "RUSTUS_SERVER_HOST")]
    pub host: String,

    /// Rustus server port
    #[arg(long, default_value = "1081", env = "RUSTUS_SERVER_PORT")]
    pub port: u16,

    #[arg(long, env = "RUSTUS_DISABLE_HEALTH_ACCESS_LOG")]
    pub disable_health_access_log: bool,

    /// Rustus base API url
    #[arg(long, default_value = "/files", env = "RUSTUS_URL")]
    pub url: String,

    /// Allowed hosts for CORS protocol.
    ///
    /// By default all hosts are allowed.
    #[arg(long, env = "RUSTUS_CORS", use_value_delimiter = true)]
    pub cors: Vec<String>,

    /// Maximum payload size.
    ///
    /// This limit used to reduce amount of consumed memory.
    #[arg(
        long,
        short = 'm',
        default_value = "262144",
        env = "RUSTUS_MAX_BODY_SIZE"
    )]
    pub max_body_size: usize,

    /// Rustus maximum log level
    #[arg(long, default_value = "INFO", env = "RUSTUS_LOG_LEVEL")]
    pub log_level: log::LevelFilter,

    /// Number of actix workers default value = number of cpu cores.
    #[arg(long, short, env = "RUSTUS_WORKERS")]
    pub workers: Option<usize>,

    /// Enabled extensions for TUS protocol.
    #[arg(
        long,
        default_value = "getting,creation,termination,creation-with-upload,creation-defer-length,concatenation,checksum",
        env = "RUSTUS_TUS_EXTENSIONS",
        use_value_delimiter = true
    )]
    pub tus_extensions: Vec<Extensions>,

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

    /// Maximum size of file that can be uploaded.
    ///
    /// If not set, file size is unlimited.
    #[arg(long, env = "RUSTUS_MAX_FILE_SIZE")]
    pub max_file_size: Option<usize>,

    #[command(flatten)]
    pub storage_opts: DataStorageOptions,

    #[command(flatten)]
    pub info_storage_opts: InfoStoreOptions,

    #[command(flatten)]
    pub notification_opts: NotificationsOptions,

    #[command(flatten)]
    pub sentry_opts: SentryOptions,
}

impl RustusConf {
    /// Function to parse CLI parametes.
    ///
    /// This is a workaround for issue mentioned
    /// [here](https://www.reddit.com/r/rust/comments/8ddd19/confusion_with_splitting_mainrs_into_smaller/).
    pub fn from_args() -> RustusConf {
        let mut conf = RustusConf::parse();
        conf.normalize_extentions();
        conf
    }

    pub fn from_iter<I>(iter: I) -> RustusConf
    where
        I: IntoIterator,
        I::Item: Into<OsString> + Clone,
    {
        <RustusConf as Parser>::parse_from(iter)
    }

    /// Base API url.
    pub fn base_url(&self) -> String {
        let stripped_prefix = self.url.strip_prefix('/').unwrap_or(self.url.as_str());
        String::from(stripped_prefix.strip_suffix('/').unwrap_or(stripped_prefix))
    }

    /// Helper for generating URI for test files.
    #[cfg(test)]
    pub fn file_url(&self, file_id: &str) -> String {
        format!("/{}/{}/", self.base_url(), file_id)
    }

    #[cfg(test)]
    pub fn test_url(&self) -> String {
        format!("/{}/", self.base_url())
    }

    /// Check if hook is enabled by user.
    pub fn hook_is_active(&self, hook: Hook) -> bool {
        self.notification_opts.hooks.contains(&hook)
    }

    /// Normalize extension vec.
    ///
    ///  Nomralization consists of two parts:
    ///  1. Adding dependent extentions (e.g. creation-with-upload depends on creation);
    ///  2. Sorting the resulting extentions;
    ///
    /// Protocol extensions must be sorted,
    /// because Actix doesn't override
    /// existing methods.
    pub fn normalize_extentions(&mut self) {
        let ext = &mut self.tus_extensions;
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
    }
}
