use crate::error::RVocResult;
use crate::{configuration::Configuration, error::RVocError};
use clap::Parser;
use database::setup_database;
use tracing::{debug, info, instrument};

mod configuration;
mod database;
mod error;
mod schema;

/// Decide how to run the application.
/// This should only be used internally for code that does not support async,
/// and hence should be run as subprocess.
#[derive(Parser, Debug)]
enum Cli {
    /// Run the web API, this is the only variant that should be called by the user.
    Web,
    /// Update the wiktionary data.
    UpdateWiktionary,
}

#[instrument(err, skip(configuration))]
fn setup_tracing_subscriber(configuration: &Configuration) -> RVocResult<()> {
    use opentelemetry::sdk::Resource;
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use tracing::subscriber::set_global_default;
    use tracing_subscriber::fmt::Layer;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    let logging_layer = Layer::default().json().with_span_list(true);
    let subscriber = Registry::default().with(logging_layer);

    let with_otel = if let Some(opentelemetry_url) = configuration.opentelemetry_url.as_ref() {
        let tracer =
            opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_trace_config(opentelemetry::sdk::trace::config().with_resource(
                    Resource::new(vec![KeyValue::new("service.name", "rvoc-backend")]),
                ))
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint(opentelemetry_url),
                )
                .install_batch(opentelemetry::runtime::TokioCurrentThread)
                .map_err(|error| RVocError::SetupTracing {
                    cause: Box::new(error),
                })?;

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        set_global_default(subscriber.with(otel_layer)).map(|_| true)
    } else {
        set_global_default(subscriber).map(|_| false)
    }
    .map_err(|error| RVocError::SetupTracing {
        cause: Box::new(error),
    })?;

    info!(
        "Set up tracing subscriber successfully {}",
        if with_otel {
            "including opentelemetry"
        } else {
            "without opentelemetry"
        }
    );

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> RVocResult<()> {
    // Load configuration & CLI
    let configuration = Configuration::from_environment()?;
    let cli = Cli::parse();
    debug!("Cli arguments: {cli:#?}");

    setup_tracing_subscriber(&configuration)?;

    run_rvoc_backend(&configuration).await?;

    Ok(())
}

#[instrument(err, skip(configuration))]
async fn run_rvoc_backend(configuration: &Configuration) -> RVocResult<()> {
    debug!("Running rvoc backend with configuration: {configuration:#?}");

    let _db_connection_pool = setup_database(configuration).await?;

    Ok(())
}
