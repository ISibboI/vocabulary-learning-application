use std::sync::{atomic, Arc};

use clap::Parser;
use tracing::{debug, info, instrument};

use crate::{
    configuration::Configuration,
    database::{
        create_async_database_connection_pool,
        migrations::{has_missing_migrations, run_migrations},
    },
    error::RVocError,
    error::RVocResult,
    job_queue::{jobs::update_witkionary::run_update_wiktionary, spawn_job_queue_runner},
    web::run_web_api,
};

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
    /// Expire the passwords of all users.
    ExpireAllPasswords,
}

#[instrument(skip(configuration))]
pub async fn run_cli_command(configuration: &Configuration) -> RVocResult<()> {
    let cli_command = Cli::parse();
    debug!("Cli arguments: {cli_command:#?}");

    match cli_command {
        Cli::Web => run_rvoc_backend(configuration).await?,
        Cli::UpdateWiktionary => {
            run_update_wiktionary(
                &create_async_database_connection_pool(configuration).await?,
                configuration,
            )
            .await?
        }
        Cli::ApplyMigrations => apply_pending_database_migrations(configuration).await?,
        Cli::ExpireAllPasswords => expire_all_passwords(configuration).await?,
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

#[instrument(err, skip(configuration))]
async fn expire_all_passwords(configuration: &Configuration) -> RVocResult<()> {
    todo!()

    //Ok(())
}
