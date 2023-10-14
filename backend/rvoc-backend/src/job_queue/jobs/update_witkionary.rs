use tracing::warn;

use crate::{
    configuration::Configuration, database::RVocAsyncDatabaseConnectionPool, error::RVocResult,
    update_wiktionary::run_update_wiktionary,
};

pub async fn update_wiktionary(
    database_connection_pool: &RVocAsyncDatabaseConnectionPool,
    configuration: &Configuration,
) -> RVocResult<()> {
    if configuration.integration_test_mode {
        warn!("Not running update_wiktionary because integration_test_mode is enabled");
        return Ok(());
    }

    run_update_wiktionary(database_connection_pool, configuration).await
}
