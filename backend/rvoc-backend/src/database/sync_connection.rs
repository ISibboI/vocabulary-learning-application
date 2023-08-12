use diesel::PgConnection;
use tracing::instrument;

use crate::{
    configuration::Configuration,
    error::{RVocError, RVocResult},
};

#[instrument(err, skip(configuration))]
pub fn create_sync_connection(configuration: &Configuration) -> RVocResult<PgConnection> {
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
