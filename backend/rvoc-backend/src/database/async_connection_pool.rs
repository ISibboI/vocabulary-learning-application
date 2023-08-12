use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use tracing::instrument;

use crate::{configuration::Configuration, error::RVocResult};

#[derive(Clone)]
pub struct RVocAsyncDatabaseConnectionPool {
    pub(super) implementation: Pool<AsyncPgConnection>,
}

impl RVocAsyncDatabaseConnectionPool {
    #[instrument(err, skip(configuration))]
    pub(super) fn new(configuration: &Configuration) -> RVocResult<Self> {
        // create a new connection pool with the default config
        let connection_manager = diesel_async::pooled_connection::AsyncDieselConnectionManager::<
            diesel_async::AsyncPgConnection,
        >::new(
            std::str::from_utf8(configuration.postgres_url.unsecure())
                .expect("postgres_url should be utf8"),
        );
        let pool = Pool::builder(connection_manager).build()?;

        Ok(Self {
            implementation: pool,
        })
    }
}
