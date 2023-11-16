#![allow(async_fn_in_trait)]

use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
use log::LevelFilter;

use crate::{config::Config, server::start_server};

pub mod config;
pub mod data_storage;
pub mod errors;
pub mod info_storages;
pub mod models;
pub mod server;
pub mod state;
pub mod utils;
pub mod extensions;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;


#[cfg_attr(coverage, no_coverage)]
fn setup_logging(app_config: &Config) -> anyhow::Result<()> {
    let colors = ColoredLevelConfig::new()
        // use builder methods
        .info(Color::Green)
        .warn(Color::Yellow)
        .debug(Color::BrightCyan)
        .error(Color::BrightRed)
        .trace(Color::Blue);

    Dispatch::new()
        .level(app_config.log_level)
        .level_for("rbatis", LevelFilter::Error)
        .chain(std::io::stdout())
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S%:z]"),
                colors.color(record.level()),
                message
            ));
        })
        .apply()?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Config::parse();
    setup_logging(&args)?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(start_server(args))?;
    Ok(())
}
