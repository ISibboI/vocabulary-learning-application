use std::sync::{atomic, Arc};

use crate::database::migrations::run_migrations;
use crate::error::RVocResult;
use crate::job_queue::spawn_job_queue_runner;
use crate::web::run_web_api;
use crate::{configuration::Configuration, error::RVocError};
use clap::Parser;
use database::create_async_database_connection_pool;
use database::migrations::has_missing_migrations;
use secstr::SecVec;
use tracing::{debug, info, instrument, Level};
use tracing_subscriber::filter::FilterFn;
use tracing_subscriber::Layer;
use update_wiktionary::run_update_wiktionary;

mod configuration;
mod database;
mod error;
mod job_queue;
mod update_wiktionary;
mod web;

type SecBytes = SecVec<u8>;

/// Decide how to run the application.
/// This should only be used internally for code that does not support async,
/// and hence should be run as subprocess.
#[derive(Parser, Debug)]
enum Cli {
    /// Run the web API, this is the only variant that should be called by the user.
    Web,
    /// Update the wiktionary data.
    UpdateWiktionary,
    /// Apply pending database migrations.
    ApplyMigrations,
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

    let logging_layer = Layer::default()
        .json()
        .with_span_list(true)
        .with_filter(FilterFn::new(|metadata| {
            if metadata.target().starts_with("tokio_util::") {
                metadata.level() < &Level::TRACE
            } else if metadata.target().starts_with("hyper::") {
                metadata.level() < &Level::DEBUG
            } else {
                true
            }
        }));
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
                    source: Box::new(error),
                })?;

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        set_global_default(subscriber.with(otel_layer)).map(|_| true)
    } else {
        set_global_default(subscriber).map(|_| false)
    }
    .map_err(|error| RVocError::SetupTracing {
        source: Box::new(error),
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
async fn main() -> RVocResult<()> {
    // Load configuration & CLI
    let configuration = Configuration::from_environment()?;
    let cli = Cli::parse();
    debug!("Cli arguments: {cli:#?}");

    setup_tracing_subscriber(&configuration)?;

    match cli {
        Cli::Web => run_rvoc_backend(&configuration).await?,
        Cli::UpdateWiktionary => {
            run_update_wiktionary(
                &create_async_database_connection_pool(&configuration).await?,
                &configuration,
            )
            .await?
        }
        Cli::ApplyMigrations => apply_pending_database_migrations(&configuration).await?,
    }

    Ok(())
}

#[instrument(err, skip(configuration))]
async fn run_rvoc_backend(configuration: &Configuration) -> RVocResult<()> {
    debug!("Running rvoc backend with configuration: {configuration:#?}");

    // Connect to database.
    // (This does not actually connect to the database, connections are created lazily.)
    let database_connection_pool = create_async_database_connection_pool(configuration).await?;

    // Create shutdown flag.
    let do_shutdown = Arc::new(atomic::AtomicBool::new(false));

    // Start job queue
    let job_queue_join_handle: tokio::task::JoinHandle<Result<(), RVocError>> =
        spawn_job_queue_runner(
            database_connection_pool.clone(),
            do_shutdown.clone(),
            configuration.clone(),
        )
        .await?;

    // Start web API
    run_web_api(database_connection_pool, configuration).await?;

    // Shutdown
    info!("Shutting down...");
    do_shutdown.store(true, atomic::Ordering::Relaxed);

    info!("Waiting for asynchronous tasks to finish...");
    job_queue_join_handle
        .await
        .map_err(|error| RVocError::TokioTaskJoin {
            source: Box::new(error),
        })??;

    Ok(())
}

#[instrument(err, skip(configuration))]
async fn apply_pending_database_migrations(configuration: &Configuration) -> RVocResult<()> {
    if has_missing_migrations(configuration)? {
        info!("Executing missing database migrations");
        run_migrations(configuration)?;
        info!("Success!");
    } else {
        info!("No missing migrations");
    }

    Ok(())
}
