use std::path::PathBuf;

use structopt::StructOpt;

use crate::storages::AvailableStores;

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "tuser", about = "Tus server implementation in Rust.")]
pub struct TuserConf {
    /// Tuser host
    #[structopt(short, long, default_value = "0.0.0.0")]
    pub host: String,

    /// Tuser server port
    #[structopt(short, long, default_value = "1081")]
    pub port: u16,

    /// Tuser base API url
    #[structopt(long, default_value = "/files")]
    pub url: String,

    /// Tuser data directory
    #[structopt(long, default_value = "./data")]
    pub data: PathBuf,

    /// Enabled hooks for http events
    #[structopt(long, default_value = "pre-create,post-finish")]
    pub enabled_hooks: String,

    /// Tuser maximum log level
    #[structopt(long, default_value = "INFO")]
    pub log_level: log::LevelFilter,

    /// Storage type
    #[structopt(long, short, default_value = "file_storage")]
    pub storage: AvailableStores,

    /// Number of actix workers default value = number of cpu cores.
    #[structopt(long, short)]
    pub workers: Option<usize>,
}

impl TuserConf {
    pub fn from_args() -> TuserConf {
        <TuserConf as StructOpt>::from_args()
    }
}
