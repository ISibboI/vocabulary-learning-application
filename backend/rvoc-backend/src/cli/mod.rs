use std::sync::{atomic, Arc};

use clap::Parser;
use diesel_async::RunQueryDsl;
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

/// CLI of the vocabulary learning application.
#[derive(Parser, Debug, Default)]
enum Cli {
    /// Run the web API (default).
    #[default]
    Web,

    /// Update the wiktionary data.
    /// This is done automatically while the web application runs.
    /// But if required, it can be run manually with this command.
    ///
    /// **WARNING:** The web API should not run while running this command, since otherwise a simultaneous update is possible, with unforseeable results.
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

    // Create database connection pool.
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
    // Create database connection pool.
    // (This does not actually connect to the database, connections are created lazily.)
    let database_connection_pool = create_async_database_connection_pool(configuration).await?;

    database_connection_pool
        .execute_read_committed_transaction(
            |database_connection| {
                Box::pin(async {
                    use crate::database::schema::users::dsl::*;
                    use diesel::ExpressionMethods;

                    diesel::update(users)
                        .set(password_hash.eq(Option::<String>::None))
                        .execute(database_connection)
                        .await
                        .map_err(|error| {
                            RVocError::ExpireAllPasswords {
                                source: Box::new(error),
                            }
                            .into()
                        })
                })
            },
            configuration.maximum_transaction_retry_count,
        )
        .await?;

    todo!()

    //Ok(())
}
