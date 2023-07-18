use crate::configuration::{parse_configuration, Configuration};
use crate::error::RVocResult;
use log::{error, info};
use std::time::Duration;
use tokio::runtime::Builder;

pub fn main() {
    let configuration =
        parse_configuration().unwrap_or_else(|e| panic!("Cannot parse configuration: {:?}", e));

    info!("Building tokio runtime...");
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| panic!("Cannot create tokio runtime: {:?}", e));
    info!("Built tokio runtime");
    info!("Entering tokio runtime...");
    runtime.block_on(async {
        run_rvoc_backend(&configuration)
            .await
            .unwrap_or_else(|e| error!("Application error: {:#?}", e));
    });

    info!("Tokio runtime returned, shutting down with timeout {}s...", configuration.tokio_shutdown_timeout_seconds);
    runtime.shutdown_timeout(Duration::from_secs(configuration.tokio_shutdown_timeout_seconds));
    info!("Tokio runtime shut down successfully");

    info!("Terminated");
}

async fn run_rvoc_backend(_configuration: &Configuration) -> RVocResult<()> {
    todo!()
}
