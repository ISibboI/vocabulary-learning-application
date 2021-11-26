use crate::api_server::run_api_server;
use crate::database::connect_to_database;
use crate::error::RVocResult;
use clap::Parser;
use log::{info, LevelFilter};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::time::Duration;
use tokio::runtime::Builder;

mod api_server;
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

#[derive(Parser, Clone)]
#[clap(version = "0.1.0", author = "Sebastian Schmidt <isibboi@gmail.com>")]
pub struct Configuration {
    #[clap(default_value = "1")]
    tokio_worker_threads: usize,

    #[clap(
        default_value = "5",
        about = "The shutdown timeout for the tokio runtime in seconds"
    )]
    tokio_shutdown_timeout: u64,

    #[clap(default_value = "mongodb://root:test@localhost:27017")]
    mongodb_uri: String,

    #[clap(default_value = "localhost")]
    mongodb_host: String,

    #[clap(default_value = "27017")]
    mongodb_port: u16,

    #[clap(default_value = "root")]
    mongodb_user: String,

    #[clap(default_value = "test")]
    mongodb_password: String,

    #[clap(default_value = "rvoc")]
    mongodb_database: String,

    #[clap(default_value = "0.0.0.0")]
    api_listen_address: String,

    #[clap(default_value = "2374")]
    api_listen_port: u16,
}

fn main() {
    init_logging();

    let configuration: Configuration = Configuration::parse();

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
