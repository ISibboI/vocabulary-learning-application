use crate::configuration::Configuration;
use crate::error::RVocResult;
use log::{error, info};
use tokio::runtime::Builder;

mod configuration;
mod error;

pub fn main() -> RVocResult<()> {
    let configuration = Configuration::from_environment()?;

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

    info!(
        "Tokio runtime returned, shutting down with timeout {}s...",
        configuration.shutdown_timeout.as_secs_f32(),
    );
    runtime.shutdown_timeout(configuration.shutdown_timeout);
    info!("Tokio runtime shut down successfully");

    info!("Terminated");
    Ok(())
}

async fn run_rvoc_backend(_configuration: &Configuration) -> RVocResult<()> {
    todo!()
}
