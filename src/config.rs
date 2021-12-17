use std::path::PathBuf;

use structopt::StructOpt;

use crate::errors::TuserError;
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

    /// Enabled extensions for TUS protocol.
    #[structopt(long, default_value = "creation,creation-with-upload")]
    pub extensions: String,
}

/// Enum of available Protocol Extensions
#[derive(PartialEq, PartialOrd, Ord, Eq)]
pub enum ProtocolExtensions {
    CreationWithUpload,
    Creation,
    Termination,
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
            _ => Err(TuserError::UnknownExtension(value.clone())),
        }
    }
}

impl From<ProtocolExtensions> for String {
    /// Mapping protocol extensions to their
    /// original names.
    fn from(ext: ProtocolExtensions) -> Self {
        match ext {
            ProtocolExtensions::Creation => Self::from("creation"),
            ProtocolExtensions::CreationWithUpload => Self::from("creation-with-upload"),
            ProtocolExtensions::Termination => Self::from("termination"),
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
