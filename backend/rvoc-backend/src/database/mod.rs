use crate::{
    configuration::Configuration,
    error::{RVocError, RVocResult},
};
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use tracing::{debug, info, instrument};

pub async fn setup_database(configuration: &Configuration) -> RVocResult<Pool<AsyncPgConnection>> {
    run_migrations(configuration)?;

    create_async_connection_pool(configuration).await
}

#[instrument(err, skip(configuration))]
fn run_migrations(configuration: &Configuration) -> RVocResult<()> {
    use diesel::Connection;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

    // Needs to be a sync connection, because `diesel_migrations` does not support async yet,
    // and `diesel_async` does not support migrations yet.
    debug!("Creating synchronous connection to database");
    let mut conn = diesel::PgConnection::establish(
        std::str::from_utf8(configuration.postgres_url.unsecure())
            .expect("postgres_url should be utf8"),
    )
    .map_err(|error| RVocError::DatabaseMigration {
        cause: Box::new(error),
    })?;
    info!("Running pending database migrations (this may take a long time)...");
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|error| RVocError::DatabaseMigration { cause: error })?;
    info!("Database migrations complete");
    Ok(())
}

#[instrument(err, skip(configuration))]
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
