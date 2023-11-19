#![allow(async_fn_in_trait)]

use errors::RustusResult;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
use log::LevelFilter;

pub mod config;
pub mod data_storage;
pub mod errors;
pub mod extensions;
pub mod info_storages;
pub mod models;
pub mod notifiers;
pub mod server;
pub mod state;
pub mod utils;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg_attr(coverage, no_coverage)]
fn setup_logging(app_config: &config::Config) -> RustusResult<()> {
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
#[cfg_attr(coverage, no_coverage)]
fn greeting(app_conf: &config::Config) {
    let extensions = app_conf
        .tus_extensions
        .clone()
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let hooks = app_conf
        .notification_config
        .hooks
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join(", ");
    let rustus_logo = include_str!("../imgs/rustus_startup_logo.txt");
    eprintln!("\n\n{rustus_logo}");
    eprintln!("Welcome to rustus!");
    eprintln!("Base URL: {}", app_conf.get_url(""));
    eprintln!("Available extensions: {extensions}");
    eprintln!("Enabled hooks: {hooks}");
    eprintln!();
    eprintln!();
}

fn main() -> RustusResult<()> {
    let args = config::Config::parse();
    setup_logging(&args)?;
    greeting(&args);
    let mut builder = if Some(1) == args.workers {
        tokio::runtime::Builder::new_current_thread()
    } else {
        let mut mtbuilder = tokio::runtime::Builder::new_multi_thread();
        if let Some(workers) = args.workers {
            mtbuilder.worker_threads(workers);
        }
        mtbuilder
    };
    builder
        .enable_all()
        .build()?
        .block_on(server::start(args))?;
    Ok(())
}
