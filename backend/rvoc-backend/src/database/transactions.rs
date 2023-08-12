use std::error::Error;

use diesel::PgConnection;
use diesel_async::AsyncPgConnection;
use tracing::{debug, instrument};

use crate::error::{BoxDynError, RVocError};

use super::{RVocAsyncDatabaseConnectionPool, RVocSyncDatabaseConnection};

impl RVocAsyncDatabaseConnectionPool {
    /// Execute an asynchronous database transaction and retry on failure.
    /// Temporary failures are logged and the transaction is retried (by calling the closure again).
    /// Permanent failures cause the function to return immediately.
    ///
    /// If `max_retries` temporary errors have occurred, then [`PermanentError::too_many_temporary_errors`] is returned.
    #[instrument(err, skip(self, transaction))]
    pub async fn execute_transaction_with_retries<
        'b,
        ReturnType: 'b + Send,
        PermanentErrorType: PermanentTransactionError,
    >(
        &self,
        transaction: impl for<'r> Fn(
                &'r mut AsyncPgConnection,
            ) -> diesel_async::scoped_futures::ScopedBoxFuture<
                'b,
                'r,
                Result<ReturnType, TransactionError>,
            > + Sync,
        max_retries: u64,
    ) -> Result<ReturnType, PermanentErrorType> {
        let mut database_connection = self.implementation.get().await.map_err(|error| {
            PermanentErrorType::permanent_error(Box::new(RVocError::DatabaseConnection {
                source: Box::new(error),
            }))
        })?;

        for _ in 0..max_retries.saturating_add(1) {
            match database_connection
                .build_transaction()
                .serializable()
                .run(&transaction)
                .await
            {
                Ok(result) => return Ok(result),
                Err(TransactionError::Temporary(error)) => {
                    debug!("temporary transaction error: {error}")
                }
                Err(TransactionError::Permanent(error)) => {
                    return Err(PermanentErrorType::permanent_error(error))
                }
                Err(TransactionError::Diesel(error)) => {
                    return Err(PermanentErrorType::permanent_error(Box::new(error)))
                }
            }
        }

        Err(PermanentTransactionError::too_many_temporary_errors(
            max_retries,
        ))
    }
}

impl RVocSyncDatabaseConnection {
    /// Execute a synchronous database transaction and retry on failure.
    /// Temporary failures are logged and the transaction is retried (by calling the closure again).
    /// Permanent failures cause the function to return immediately.
    ///
    /// If `max_retries` temporary errors have occurred, then [`PermanentError::too_many_temporary_errors`] is returned.
    pub fn execute_sync_transaction_with_retries<
        ReturnType,
        PermanentErrorType: PermanentTransactionError,
    >(
        &mut self,
        transaction: impl Fn(&mut PgConnection) -> Result<ReturnType, TransactionError>,
        max_retries: u64,
    ) -> Result<ReturnType, PermanentErrorType> {
        for _ in 0..max_retries.saturating_add(1) {
            match self
                .implementation
                .build_transaction()
                .serializable()
                .run(&transaction)
            {
                Ok(result) => return Ok(result),
                Err(TransactionError::Temporary(error)) => {
                    debug!("temporary transaction error: {error}")
                }
                Err(TransactionError::Permanent(error)) => {
                    return Err(PermanentErrorType::permanent_error(error))
                }
                Err(TransactionError::Diesel(error)) => {
                    return Err(PermanentErrorType::permanent_error(Box::new(error)))
                }
            }
        }

        Err(PermanentTransactionError::too_many_temporary_errors(
            max_retries,
        ))
    }
}

pub enum TransactionError {
    /// The transaction was unable to complete, but should be retried.
    #[allow(unused)]
    Temporary(BoxDynError),
    /// The transaction was unable to complete and should not be retried.
    Permanent(BoxDynError),
    /// A database error.
    Diesel(diesel::result::Error),
}

impl From<BoxDynError> for TransactionError {
    fn from(value: BoxDynError) -> Self {
        Self::Permanent(value)
    }
}

impl From<diesel::result::Error> for TransactionError {
    fn from(value: diesel::result::Error) -> Self {
        Self::Diesel(value)
    }
}

/// An error type that indicates a permanent transaction failure.
pub trait PermanentTransactionError: Error {
    /// Construct the error instance representing "too many temporary errors".
    /// The `limit` is the error limit that was reached.
    fn too_many_temporary_errors(limit: u64) -> Self;

    /// Construct the error instance representing a general permanent error.
    fn permanent_error(source: BoxDynError) -> Self;
}

impl PermanentTransactionError for RVocError {
    fn too_many_temporary_errors(limit: u64) -> Self {
        Self::DatabaseTransactionRetryLimitReached { limit }
    }

    fn permanent_error(source: BoxDynError) -> Self {
        Self::PermanentDatabaseTransactionError { source }
    }
}
