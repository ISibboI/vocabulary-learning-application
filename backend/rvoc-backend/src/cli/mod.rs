use std::sync::{atomic, Arc};

use clap::Parser;
use diesel_async::RunQueryDsl;
use secure_string::{SecureBytes, SecureString};
use tokio::io::{stdin, AsyncReadExt};
use tracing::{debug, info, instrument};

use crate::{
    configuration::Configuration,
    database::{
        create_async_database_connection_pool,
        migrations::{has_missing_migrations, run_migrations},
    },
    error::RVocError,
    error::RVocResult,
    integration_tests::run_internal_integration_tests,
    job_queue::{jobs::update_witkionary::run_update_wiktionary, spawn_job_queue_runner},
    model::user::password_hash::PasswordHash,
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

    /// Expire the passwords and sessions of all users.
    /// This should always succeed, and users who update their passwords simultaneously should receive an error.
    ExpireAllPasswords,

    /// Expire all sessions of all users.
    /// This should always succeed, and sessions that are updated simultaneously should be be logged out anyways.
    ExpireAllSessions,

    /// Set the password of a user.
    /// If no password is given, then it is read from stdin.
    SetPassword {
        /// The name of the user.
        #[arg(short, long)]
        username: String,
        /// The new password.
        /// If not given, then it is read from stdin.
        #[arg(short, long)]
        password: Option<SecureBytes>,
    },

    /// Run integration tests that require a database, but use APIs that are not exposed through the web interface.
    RunInternalIntegrationTests,
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
        Cli::ExpireAllSessions => expire_all_sessions(configuration).await?,
        Cli::SetPassword { username, password } => {
            set_password(username, password, configuration).await?
        }
        Cli::RunInternalIntegrationTests => run_internal_integration_tests(configuration).await?,
    }

    Ok(())
}

#[instrument(err, skip(configuration))]
async fn run_rvoc_backend(configuration: &Configuration) -> RVocResult<()> {
    debug!("Running rvoc backend with configuration: {configuration:#?}");

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

    expire_all_sessions(configuration).await?;

    Ok(())
}

#[instrument(err, skip(configuration))]
async fn expire_all_sessions(configuration: &Configuration) -> RVocResult<()> {
    let database_connection_pool = create_async_database_connection_pool(configuration).await?;

    database_connection_pool
        .execute_read_committed_transaction(
            |database_connection| {
                Box::pin(async {
                    use crate::database::schema::sessions::dsl::*;

                    diesel::delete(sessions)
                        .execute(database_connection)
                        .await
                        .map_err(|error| {
                            RVocError::ExpireAllSessions {
                                source: Box::new(error),
                            }
                            .into()
                        })
                })
            },
            configuration.maximum_transaction_retry_count,
        )
        .await?;

    Ok(())
}

#[instrument(err, skip(configuration))]
async fn set_password(
    username: String,
    password: Option<SecureBytes>,
    configuration: &Configuration,
) -> RVocResult<()> {
    let password = if let Some(password) = password {
        password
    } else {
        let mut password = Vec::new();
        stdin().read_to_end(&mut password).await.map_err(|error| {
            RVocError::ReadPasswordFromStdin {
                source: Box::new(error),
            }
        })?;
        SecureBytes::from(password)
    };

    let password_hash = PasswordHash::new(password, configuration)?;
    let password_hash = Option::<SecureString>::from(password_hash).expect(
        "creating a password hash from a password should never return an empty password hash",
    );

    let database_connection_pool = create_async_database_connection_pool(configuration).await?;

    database_connection_pool
        .execute_transaction::<_, RVocError>(
            |database_connection| {
                Box::pin(async {
                    use crate::database::schema::users;
                    use diesel::ExpressionMethods;

                    diesel::update(users::table)
                        .filter(users::name.eq(&username))
                        .set(users::password_hash.eq(password_hash.unsecure()))
                        .execute(database_connection)
                        .await
                        .map_err(Into::into)
                })
            },
            configuration.maximum_transaction_retry_count,
        )
        .await?;

    Ok(())
}
