use std::time::Duration;

use tokio::time::sleep;
use tracing::{info, instrument};

use crate::configuration::Configuration;
use crate::database::create_async_database_connection_pool;
use crate::error::{RVocError, RVocResult};

#[instrument(err, skip(configuration))]
pub async fn run_internal_integration_tests(configuration: &Configuration) -> RVocResult<()> {
    test_aborted_transaction(configuration).await
}

#[instrument(err, skip(configuration))]
async fn test_aborted_transaction(configuration: &Configuration) -> RVocResult<()> {
    let database_connection_pool = create_async_database_connection_pool(configuration).await?;

    // Set up test table
    database_connection_pool
        .execute_transaction::<_, RVocError>(
            |database_connection| {
                Box::pin(async move {
                    use crate::database::schema::test_can_be_safely_dropped_in_production::dsl::*;
                    use diesel::ExpressionMethods;
                    use diesel_async::RunQueryDsl;

                    diesel::delete(test_can_be_safely_dropped_in_production)
                        .filter(id.eq_any([1, 2]))
                        .execute(database_connection)
                        .await?;
                    diesel::insert_into(test_can_be_safely_dropped_in_production)
                        .values([(id.eq(1), name.eq("Tim")), (id.eq(2), name.eq("Tom"))])
                        .execute(database_connection)
                        .await?;

                    Ok(())
                })
            },
            0,
        )
        .await?;

    info!("Test table set up successfully");

    // Trigger a serialisation failure
    let (first, second) = tokio::join!(
        database_connection_pool.execute_transaction::<_, RVocError>(
            |database_connection| Box::pin(async move {
                use crate::database::schema::test_can_be_safely_dropped_in_production::dsl::*;
                use diesel::ExpressionMethods;
                use diesel::OptionalExtension;
                use diesel::QueryDsl;
                use diesel_async::RunQueryDsl;

                let tim: Option<String> = test_can_be_safely_dropped_in_production
                    .select(name)
                    .filter(name.eq("Tim"))
                    .first(database_connection)
                    .await
                    .optional()?
                    .unwrap();

                sleep(Duration::from_secs(5)).await;

                diesel::update(test_can_be_safely_dropped_in_production)
                    .filter(name.eq("Tom"))
                    .set(name.eq(tim.unwrap()))
                    .execute(database_connection)
                    .await?;

                Ok(())
            }),
            0
        ),
        database_connection_pool.execute_transaction::<_, RVocError>(
            |database_connection| Box::pin(async move {
                use crate::database::schema::test_can_be_safely_dropped_in_production::dsl::*;
                use diesel::ExpressionMethods;
                use diesel::OptionalExtension;
                use diesel::QueryDsl;
                use diesel_async::RunQueryDsl;

                let tom: Option<String> = test_can_be_safely_dropped_in_production
                    .select(name)
                    .filter(name.eq("Tom"))
                    .first(database_connection)
                    .await
                    .optional()?
                    .unwrap();

                sleep(Duration::from_secs(5)).await;

                diesel::update(test_can_be_safely_dropped_in_production)
                    .filter(name.eq("Tim"))
                    .set(name.eq(tom.unwrap()))
                    .execute(database_connection)
                    .await?;

                Ok(())
            }),
            0
        ),
    );

    info!("Serialisation failure should have triggered");
    info!("First result:  {first:?}");
    info!("Second result: {second:?}");
    assert!(
        matches!(
            first,
            Err(RVocError::DatabaseTransactionRetryLimitReached { .. })
        ) || matches!(
            second,
            Err(RVocError::DatabaseTransactionRetryLimitReached { .. })
        )
    );

    // Ensure that we can still do transactions on the same data
    database_connection_pool
        .execute_transaction::<_, RVocError>(
            |database_connection| {
                Box::pin(async move {
                    use crate::database::schema::test_can_be_safely_dropped_in_production::dsl::*;
                    use diesel::ExpressionMethods;
                    use diesel_async::RunQueryDsl;

                    diesel::update(test_can_be_safely_dropped_in_production)
                        .filter(id.eq(1))
                        .set(name.eq("Tim"))
                        .execute(database_connection)
                        .await?;
                    diesel::update(test_can_be_safely_dropped_in_production)
                        .filter(id.eq(2))
                        .set(name.eq("Tom"))
                        .execute(database_connection)
                        .await?;

                    Ok(())
                })
            },
            0,
        )
        .await?;

    info!("Success! Transactions still work after serialisation failure");

    Ok(())
}
