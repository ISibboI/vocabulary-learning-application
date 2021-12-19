use crate::api_server::run_api_server;
use crate::configuration::{parse_configuration, Configuration};
use crate::database::connect_to_database;
use crate::error::RVocResult;
use log::{info, LevelFilter};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::time::Duration;
use tokio::runtime::Builder;

mod api_server;
mod configuration;
mod database;
mod error;

fn init_logging() {
    TermLogger::init(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Stdout,
        ColorChoice::Auto,
    )
    .unwrap_or_else(|e| panic!("Cannot initialize logging: {:?}", e));
    info!("Logging initialized");
}

fn main() {
    init_logging();
    let configuration =
        parse_configuration().unwrap_or_else(|e| panic!("Cannot parse configuration: {:?}", e));

    info!("Building tokio runtime...");
    let runtime = Builder::new_multi_thread()
        .thread_name_fn(|| format!("abc"))
        .worker_threads(configuration.tokio_worker_threads)
        .enable_all()
        .build()
        .unwrap_or_else(|e| panic!("Cannot create tokio runtime: {:?}", e));
    info!("Built tokio runtime");
    info!("Entering tokio runtime...");
    runtime.block_on(async {
        run_rvoc_backend(&configuration)
            .await
            .unwrap_or_else(|e| panic!("Application error: {:?}", e));
    });
    info!("Tokio runtime returned, shutting down...");
    runtime.shutdown_timeout(Duration::from_secs(configuration.tokio_shutdown_timeout));
    info!("Tokio runtime shut down successfully");

    info!("Terminated");
}

async fn run_rvoc_backend(configuration: &Configuration) -> RVocResult<()> {
    let database = connect_to_database(configuration).await?;
    run_api_server(configuration, database).await
}
