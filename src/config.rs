use clap::Parser;

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    #[arg(long, default_value = "0.0.0.0", env = "RUSTUS_SERVER_HOST")]
    pub host: String,
    #[arg(long, default_value = "1081", env = "RUSTUS_SERVER_PORT")]
    pub port: u16,
    #[arg(long, default_value = "INFO", env = "RUSTUS_LOG_LEVEL")]
    pub log_level: log::LevelFilter,
    #[arg(long, default_value = "/", env = "RUSTUS_PREFIX")]
    pub base_path: String,
}
