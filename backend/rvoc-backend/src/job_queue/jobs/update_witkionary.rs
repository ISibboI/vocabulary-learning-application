use crate::{
    configuration::Configuration, database::RVocAsyncDatabaseConnectionPool, error::RVocResult,
    update_wiktionary::run_update_wiktionary,
};

pub async fn update_wiktionary(
    database_connection_pool: &RVocAsyncDatabaseConnectionPool,
    configuration: &Configuration,
) -> RVocResult<()> {
    run_update_wiktionary(database_connection_pool, configuration).await
}
