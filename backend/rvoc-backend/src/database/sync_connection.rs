use diesel::PgConnection;
use tracing::instrument;

use crate::{
    configuration::Configuration,
    error::{RVocError, RVocResult},
};

pub struct RVocSyncDatabaseConnection {
    pub(super) implementation: PgConnection,
}

impl RVocSyncDatabaseConnection {
    #[instrument(err, skip(configuration))]
    pub(super) fn new(configuration: &Configuration) -> RVocResult<Self> {
        use diesel::Connection;

        // create a new connection with the default config
        let connection = PgConnection::establish(
            std::str::from_utf8(configuration.postgres_url.unsecure())
                .expect("postgres_url should be utf8"),
        )
        .map_err(|error| RVocError::DatabaseConnection {
            source: Box::new(error),
        })?;
        Ok(Self {
            implementation: connection,
        })
    }

    pub(super) fn get_mut(&mut self) -> &mut PgConnection {
        &mut self.implementation
    }
}
