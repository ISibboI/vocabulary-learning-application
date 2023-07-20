use crate::error::RVocResult;
use crate::{configuration::Configuration, error::RVocError};
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use tracing::{error, info, instrument};

mod configuration;
mod error;

fn setup_tracing() {
    use tracing_subscriber::fmt::format::FmtSpan;

    tracing_subscriber::fmt()
        .json()
        .with_span_list(true)
        .with_span_events(FmtSpan::FULL)
        .init();
}

pub fn main() -> RVocResult<()> {
    // Load configuration
    let configuration = Configuration::from_environment()?;

    setup_tracing();

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
}

async fn run_rvoc_backend(configuration: &Configuration) -> RVocResult<()> {
    run_migrations(configuration)?;

    let _db_connection_pool = create_async_connection_pool(configuration).await?;

    Ok(())
}

#[instrument(err)]
pub fn run_migrations(configuration: &Configuration) -> RVocResult<()> {
    use diesel::Connection;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

    // Needs to be a sync connection, because `diesel_migrations` does not support async yet,
    // and `diesel_async` does not support migrations yet.
    info!("Creating synchronous connection to database");
    let mut conn = diesel::PgConnection::establish(
        std::str::from_utf8(configuration.postgres_url.unsecure())
            .expect("postgres_url should be utf8"),
    )
    .map_err(|error| RVocError::DatabaseMigration {
        cause: Box::new(error),
    })?;
    info!("Running Database migrations (This may take a long time)...");
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|error| RVocError::DatabaseMigration { cause: error })?;
    info!("Database migrations complete.");
    Ok(())
}

#[instrument(err)]
async fn create_async_connection_pool(
    configuration: &Configuration,
) -> RVocResult<Pool<AsyncPgConnection>> {
    // create a new connection pool with the default config
    let connection_manager = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(
        std::str::from_utf8(configuration.postgres_url.unsecure())
            .expect("postgres_url should be utf8"),
    );
    let pool = Pool::builder(connection_manager).build()?;

    Ok(pool)
}
