use crate::{
    configuration::Configuration,
    error::{RVocError, RVocResult},
};
use diesel::PgConnection;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};

use self::{
    connection::{create_async_connection_pool, create_sync_connection},
    migrations::has_missing_migrations,
};

mod connection;
pub mod migrations;
pub mod model;
pub mod schema;
pub mod transactions;

/// Create an async connection pool to the database.
///
/// If there are pending database migrations, this method returns an error.
pub async fn create_async_database_connection_pool(
    configuration: &Configuration,
) -> RVocResult<Pool<AsyncPgConnection>> {
    if has_missing_migrations(configuration)? {
        Err(RVocError::PendingDatabaseMigrations)
    } else {
        create_async_connection_pool(configuration)
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
