use crate::{
    configuration::Configuration,
    error::{RVocError, RVocResult},
};

use self::migrations::has_missing_migrations;

pub use self::async_connection_pool::RVocAsyncDatabaseConnectionPool;
pub use self::sync_connection::RVocSyncDatabaseConnection;

mod async_connection_pool;
pub mod migrations;
pub mod model;
pub mod schema;
mod sync_connection;
pub mod transactions;

/// Create an async connection pool to the database.
///
/// If there are pending database migrations, this method returns an error.
pub async fn create_async_database_connection_pool(
    configuration: &Configuration,
) -> RVocResult<RVocAsyncDatabaseConnectionPool> {
    if has_missing_migrations(configuration)? {
        Err(RVocError::PendingDatabaseMigrations)
    } else {
        RVocAsyncDatabaseConnectionPool::new(configuration)
    }
}

/// Create a sync connection to the database.
///
/// If there are pending database migrations, this method returns an error.
#[allow(dead_code)]
pub fn create_sync_database_connection(
    configuration: &Configuration,
) -> RVocResult<RVocSyncDatabaseConnection> {
    if has_missing_migrations(configuration)? {
        Err(RVocError::PendingDatabaseMigrations)
    } else {
        RVocSyncDatabaseConnection::new(configuration)
    }
}
