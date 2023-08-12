use crate::{
    configuration::Configuration,
    error::{RVocError, RVocResult},
};
use diesel::PgConnection;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use tracing::{debug, info, instrument};

pub mod model;
pub mod schema;
pub mod transactions;

const MIGRATIONS: diesel_migrations::EmbeddedMigrations = diesel_migrations::embed_migrations!();

/// Create an async connection pool to the database.
///
/// If there are pending database migrations, this method returns an error.
pub async fn create_async_database_connection_pool(
    configuration: &Configuration,
) -> RVocResult<Pool<AsyncPgConnection>> {
    if has_missing_migrations(configuration)? {
        Err(RVocError::PendingDatabaseMigrations)
    } else {
        create_async_connection_pool(configuration).await
    }
}

/// Create a sync connection to the database.
///
/// If there are pending database migrations, this method returns an error.
pub fn create_sync_database_connection(configuration: &Configuration) -> RVocResult<PgConnection> {
    if has_missing_migrations(configuration)? {
        Err(RVocError::PendingDatabaseMigrations)
    } else {
        create_sync_connection(configuration)
    }
}

/// Synchronously check if there are missing database migrations.
pub fn has_missing_migrations(configuration: &Configuration) -> RVocResult<bool> {
    use diesel_migrations::MigrationHarness;

    // Needs to be a sync connection, because `diesel_migrations` does not support async yet,
    // and `diesel_async` does not support migrations yet.
    debug!("Creating synchronous connection to database");
    let mut connection = create_sync_connection(configuration)?;

    connection
        .has_pending_migration(MIGRATIONS)
        .map_err(|error| RVocError::DatabaseMigration { source: error })
}

/// Runs all missing migrations synchronously.
///
/// **Warning:** It is unknown how this deals with concurrent execution of migrations,
/// so make sure that this is never run twice at the same time on the same database.
#[instrument(err, skip(configuration))]
pub fn run_migrations(configuration: &Configuration) -> RVocResult<()> {
    use diesel_migrations::MigrationHarness;

    // Needs to be a sync connection, because `diesel_migrations` does not support async yet,
    // and `diesel_async` does not support migrations yet.
    debug!("Creating synchronous connection to database");
    let mut connection = create_sync_connection(configuration)?;
    info!("Running pending database migrations (this may take a long time)...");
    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|error| RVocError::DatabaseMigration { source: error })?;
    info!("Database migrations complete");
    Ok(())
}

#[instrument(err, skip(configuration))]
async fn create_async_connection_pool(
    configuration: &Configuration,
) -> RVocResult<Pool<AsyncPgConnection>> {
    // create a new connection pool with the default config
    let connection_manager = diesel_async::pooled_connection::AsyncDieselConnectionManager::<
        diesel_async::AsyncPgConnection,
    >::new(
        std::str::from_utf8(configuration.postgres_url.unsecure())
            .expect("postgres_url should be utf8"),
    );
    let pool = Pool::builder(connection_manager).build()?;

    Ok(pool)
}

#[instrument(err, skip(configuration))]
fn create_sync_connection(configuration: &Configuration) -> RVocResult<PgConnection> {
    use diesel::Connection;

    // create a new connection with the default config
    let connection = PgConnection::establish(
        std::str::from_utf8(configuration.postgres_url.unsecure())
            .expect("postgres_url should be utf8"),
    )
    .map_err(|error| RVocError::DatabaseConnection {
        source: Box::new(error),
    })?;
    Ok(connection)
}
