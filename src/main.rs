#![allow(async_fn_in_trait)]

use std::str::FromStr;

use errors::RustusResult;
use sentry::types::Dsn;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
    let mut _guard = None;
    if let Some(sentry_dsn) = &args.sentry_config.sentry_dsn {
        _guard = Some(sentry::init(sentry::ClientOptions {
            dsn: Dsn::from_str(sentry_dsn.as_str()).ok(),
            // Enable capturing of traces; set this a to lower value in production:
            traces_sample_rate: 1.0,
            ..sentry::ClientOptions::default()
        }));
    }

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::filter_fn(move |a| {
            a.level() <= &args.log_level
        }))
        .with(
            tracing_subscriber::fmt::layer()
                .with_level(true)
                .with_file(false)
                .with_line_number(false)
                .with_target(false)
                .compact(),
        )
        .with(sentry_tracing::layer())
        .init();

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
