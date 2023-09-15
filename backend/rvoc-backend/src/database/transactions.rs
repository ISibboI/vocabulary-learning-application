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
    pub async fn execute_transaction<
        'b,
        ReturnType: 'b + Send,
        PermanentErrorType: PermanentTransactionError + TooManyTemporaryTransactionErrors,
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
                Err(TransactionError::Diesel(
                    error @ diesel::result::Error::DatabaseError(
                        diesel::result::DatabaseErrorKind::SerializationFailure,
                        _,
                    ),
                )) => {
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

        Err(PermanentErrorType::too_many_temporary_errors(max_retries))
    }

    /// Execute an asynchronous database transaction.
    #[instrument(err, skip(self, transaction))]
    #[deprecated(
        note = "postgres transactions may fail randomly because of some optimisations in postgres"
    )]
    pub async fn execute_transaction_without_retries<
        'b,
        ReturnType: 'b + Send,
        ErrorType: 'b + Send + PermanentTransactionError,
    >(
        &self,
        transaction: impl for<'r> FnOnce(
                &'r mut AsyncPgConnection,
            ) -> diesel_async::scoped_futures::ScopedBoxFuture<
                'b,
                'r,
                Result<ReturnType, ErrorType>,
            > + Send
            + Sync,
    ) -> Result<ReturnType, ErrorType> {
        let mut database_connection = self.implementation.get().await.map_err(|error| {
            ErrorType::permanent_error(Box::new(RVocError::DatabaseConnection {
                source: Box::new(error),
            }))
        })?;

        database_connection
            .build_transaction()
            .serializable()
            .run(|database_connection| {
                Box::pin(async move {
                    transaction(database_connection)
                        .await
                        .map_err(|error| FromDieselError::ErrorType(error))
                })
            })
            .await
            .map_err(|error| match error {
                FromDieselError::ErrorType(error) => error,
                FromDieselError::Diesel(error) => ErrorType::permanent_error(Box::new(error)),
            })
    }
}

impl RVocSyncDatabaseConnection {
    /// Execute a synchronous database transaction and retry on failure.
    /// Temporary failures are logged and the transaction is retried (by calling the closure again).
    /// Permanent failures cause the function to return immediately.
    ///
    /// If `max_retries` temporary errors have occurred, then [`PermanentError::too_many_temporary_errors`] is returned.
    #[allow(dead_code)]
    pub fn execute_transaction<
        ReturnType,
        PermanentErrorType: PermanentTransactionError + TooManyTemporaryTransactionErrors,
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
                Err(TransactionError::Diesel(
                    error @ diesel::result::Error::DatabaseError(
                        diesel::result::DatabaseErrorKind::SerializationFailure,
                        _,
                    ),
                )) => {
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

        Err(PermanentErrorType::too_many_temporary_errors(max_retries))
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
    /// Construct the error instance representing a general permanent error.
    fn permanent_error(source: BoxDynError) -> Self;
}

impl PermanentTransactionError for RVocError {
    fn permanent_error(source: BoxDynError) -> Self {
        Self::PermanentDatabaseTransactionError { source }
    }
}

/// An error type that indicates a permanent transaction failure caused by too many temporary failures.
pub trait TooManyTemporaryTransactionErrors: Error {
    /// Construct the error instance representing "too many temporary errors".
    /// The `limit` is the error limit that was reached.
    fn too_many_temporary_errors(limit: u64) -> Self;
}

impl TooManyTemporaryTransactionErrors for RVocError {
    fn too_many_temporary_errors(limit: u64) -> Self {
        Self::DatabaseTransactionRetryLimitReached { limit }
    }
}

enum FromDieselError<ErrorType> {
    ErrorType(ErrorType),
    Diesel(diesel::result::Error),
}

impl<ErrorType> From<diesel::result::Error> for FromDieselError<ErrorType> {
    fn from(value: diesel::result::Error) -> Self {
        Self::Diesel(value)
    }
}
