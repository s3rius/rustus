use chrono::format;
use clap::Parser;

use crate::{extensions::TusExtensions, info_storages::AvailableInfoStorages};

#[derive(Parser, Clone, Debug)]
pub struct InfoStorageConfig {
    #[arg(long, default_value = "file", env = "RUSTUS_INFO_STORAGE")]
    pub info_storage: AvailableInfoStorages,
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

    /// Maximum size of file that can be uploaded.
    ///
    /// If not set, file size is unlimited.
    #[arg(long, env = "RUSTUS_MAX_FILE_SIZE")]
    pub max_file_size: Option<usize>,

    #[command(flatten)]
    pub info_storage_config: InfoStorageConfig,

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

    pub fn get_url(&self, url: String) -> String {
        format!("{}/{url}", self.url)
    }
}
