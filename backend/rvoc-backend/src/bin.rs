use crate::configuration::Configuration;
use crate::error::RVocResult;
use database::setup_database;
use tracing::{error, info};

mod configuration;
mod database;
mod error;

fn setup_tracing_subscriber() -> impl tracing::Subscriber {
    use opentelemetry::sdk::export::trace::stdout;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    let tracer = stdout::new_pipeline().install_simple();
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    Registry::default().with(telemetry)
}

pub fn main() -> RVocResult<()> {
    // Load configuration
    let configuration = Configuration::from_environment()?;

    let subscriber = setup_tracing_subscriber();

    tracing::subscriber::with_default(subscriber, || {
        info!("Building tokio runtime...");
        let runtime = tokio::runtime::Builder::new_current_thread()
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
    })
}

async fn run_rvoc_backend(configuration: &Configuration) -> RVocResult<()> {
    let _db_connection_pool = setup_database(configuration).await?;

    Ok(())
}
