use clap::Parser;
use log::{info, LevelFilter};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::time::Duration;
use tokio::runtime::Builder;
use wither::mongodb::Client;
use crate::error::RVocResult;

mod error;

fn init_logging() {
    TermLogger::init(
        LevelFilter::Trace,
        Config::default(),
        TerminalMode::Stdout,
        ColorChoice::Auto,
    )
    .unwrap_or_else(|e| panic!("Cannot initialize logging: {:?}", e));
    info!("Logging initialized");
}

#[derive(Parser, Clone)]
#[clap(version = "0.1.0", author = "Sebastian Schmidt <isibboi@gmail.com>")]
struct Configuration {
    #[clap(default_value = "1")]
    tokio_worker_threads: usize,

    #[clap(
        default_value = "5",
        about = "The shutdown timeout for the tokio runtime in seconds"
    )]
    tokio_shutdown_timeout: u64,

    #[clap(default_value = "mongodb://localhost:27017")]
    mongodb_uri: String,

    #[clap(default_value = "root")]
    mongodb_user: String,

    #[clap(default_value = "test")]
    mongodb_password: String,

    #[clap(default_value = "rvoc")]
    mongodb_database: String,
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
        run_rvoc_backend(configuration.clone()).await.unwrap_or_else(|e| panic!("Application error: {:?}", e));
    });
    info!("Tokio runtime returned, shutting down...");
    runtime.shutdown_timeout(Duration::from_secs(configuration.tokio_shutdown_timeout));
    info!("Tokio runtime shut down successfully");

    info!("Terminated");
}

async fn run_rvoc_backend(configuration: Configuration) -> RVocResult<()> {
    info!("Connecting to mongodb database '{}' at '{}'...", configuration.mongodb_database, configuration.mongodb_uri);
    let database = Client::with_uri_str(&configuration.mongodb_uri)
        .await?
        .database(&configuration.mongodb_database);
    info!("Connection established successfully");

    Ok(())
}
