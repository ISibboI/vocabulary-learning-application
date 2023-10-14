use crate::{
    configuration::Configuration,
    database::RVocAsyncDatabaseConnectionPool,
    error::{RVocError, RVocResult},
};

pub async fn delete_expired_sessions(
    database_connection_pool: &RVocAsyncDatabaseConnectionPool,
    configuration: &Configuration,
) -> RVocResult<()> {
    // We execute this in read-committed mode to hopefully make it never fail,
    // since it potentially touches a lot of rows. Rather, if a session gets
    // deleted while another transaction updates it, we don't care and delete it anyways.
    database_connection_pool
        .execute_read_committed_transaction::<_, RVocError>(
            |database_connection| {
                Box::pin(async {
                    use crate::database::schema::sessions::dsl::*;
                    use diesel::dsl::now;
                    use diesel::ExpressionMethods;
                    use diesel::QueryDsl;
                    use diesel_async::RunQueryDsl;

                    diesel::delete(sessions.filter(expiry.lt(now)))
                        .execute(database_connection)
                        .await?;

                    Ok(())
                })
            },
            configuration.maximum_transaction_retry_count,
        )
        .await
}
