use tracing::{debug, info, instrument};

use crate::{
    configuration::Configuration,
    database::sync_connection::create_sync_connection,
    error::{RVocError, RVocResult},
};

const MIGRATIONS: diesel_migrations::EmbeddedMigrations = diesel_migrations::embed_migrations!();

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
