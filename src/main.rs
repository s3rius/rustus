#![allow(async_fn_in_trait)]

use std::{borrow::Cow, str::FromStr};

use errors::RustusResult;

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
    #[allow(clippy::no_effect_underscore_binding)]
    let mut _guard = None;
    if let Some(sentry_dsn) = &args.sentry_config.dsn {
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
