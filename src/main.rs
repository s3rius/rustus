#![allow(async_fn_in_trait)]
#![warn(
    // Base lints.
    clippy::all,
    // Some pedantic lints.
    clippy::pedantic,
    // New lints which are cool.
    clippy::nursery,
)]
#![
    allow(
        // I don't care about this.
        clippy::module_name_repetitions,
        // Yo, the hell you should put
        // it in docs, if signature is clear as sky.
        clippy::missing_errors_doc
    )
]
use std::{borrow::Cow, str::FromStr};

use errors::RustusResult;
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

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

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
    greeting(&args);
    #[allow(clippy::collection_is_never_read)]
    let mut _guard = None;
    let mut sentry_layer = None;
    if let Some(sentry_dsn) = &args.sentry_config.dsn {
        sentry_layer = Some(sentry_tracing::layer());
        let default_options = sentry::ClientOptions::default();
        _guard = Some(sentry::init(sentry::ClientOptions {
            dsn: sentry::types::Dsn::from_str(sentry_dsn.as_str()).ok(),
            // Enable capturing of traces; set this a to lower value in production:
            sample_rate: args
                .sentry_config
                .sample_rate
                .unwrap_or(default_options.sample_rate),
            traces_sample_rate: args
                .sentry_config
                .traces_sample_rate
                .unwrap_or(default_options.traces_sample_rate),
            environment: args
                .sentry_config
                .environment
                .clone()
                .map(Cow::from)
                .clone(),
            release: sentry::release_name!(),
            debug: args.sentry_config.debug,
            ..default_options
        }));
    }

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            args.log_level,
        ))
        .with(
            tracing_subscriber::fmt::layer()
                .with_level(true)
                .with_file(false)
                .with_line_number(false)
                .with_target(false),
        )
        .with(tracing_error::ErrorLayer::default())
        .with(sentry_layer)
        .init();

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
