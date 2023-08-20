use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::Insertable;
use diesel_async::RunQueryDsl;
use thiserror::Error;
use typed_session::{Session, SessionExpiry, SessionId, WriteSessionResult};
use typed_session_axum::typed_session::SessionStoreConnector;

use crate::{
    configuration::Configuration,
    database::{
        transactions::{PermanentTransactionError, TransactionError},
        RVocAsyncDatabaseConnectionPool,
    },
    error::{BoxDynError, RVocError},
};

use super::user::Username;

#[derive(Clone)]
pub struct RVocSessionStoreConnector {
    database_connection_pool: RVocAsyncDatabaseConnectionPool,
    maximum_retries_on_id_collision: u32,
}

#[derive(Default, Debug)]
pub enum RVocSessionData {
    #[default]
    Anonymous,
    LoggedIn(Username),
}

impl RVocSessionStoreConnector {
    pub fn new(
        database_connection_pool: RVocAsyncDatabaseConnectionPool,
        configuration: &Configuration,
    ) -> Self {
        Self {
            database_connection_pool,
            maximum_retries_on_id_collision: configuration
                .maximum_session_id_generation_retry_count,
        }
    }
}

#[async_trait]
impl SessionStoreConnector<RVocSessionData> for RVocSessionStoreConnector {
    type Error = RVocError;

    fn maximum_retries_on_id_collision(&self) -> Option<u32> {
        Some(self.maximum_retries_on_id_collision)
    }

    async fn create_session(
        &mut self,
        current_id: &SessionId,
        session_expiry: &SessionExpiry,
        data: &RVocSessionData,
    ) -> Result<WriteSessionResult, typed_session::Error<Self::Error>> {
        match self
            .database_connection_pool
            .execute_transaction_with_retries::<_, TryInsertSessionError>(
                |database_connection| {
                    Box::pin(async {
                        use crate::database::schema::sessions::dsl::*;

                        RVocSessionInsertable::new(current_id, session_expiry, data)
                            .insert_into(sessions)
                            .execute(database_connection)
                            .await
                            .map_err(|error| match error {
                                diesel::result::Error::DatabaseError(
                                    diesel::result::DatabaseErrorKind::UniqueViolation,
                                    database_error_information,
                                ) => {
                                    if database_error_information.table_name() == Some("sessions")
                                        && database_error_information.column_name() == Some("id")
                                    {
                                        TryInsertSessionError::SessionIdExists
                                    } else {
                                        TryInsertSessionError::Permanent(
                                            diesel::result::Error::DatabaseError(
                                                diesel::result::DatabaseErrorKind::UniqueViolation,
                                                database_error_information,
                                            )
                                            .into(),
                                        )
                                    }
                                }
                                error => TryInsertSessionError::Permanent(error.into()),
                            })
                            .map_err(|error| TransactionError::Permanent(error.into()))?;

                        Ok(())
                    })
                },
                0,
            )
            .await
        {
            Ok(()) => Ok(WriteSessionResult::Ok(())),
            Err(TryInsertSessionError::SessionIdExists) => Ok(WriteSessionResult::SessionIdExists),
            Err(TryInsertSessionError::Permanent(error)) => {
                Err(RVocError::InsertSession { source: error })
            }
            Err(TryInsertSessionError::TooManyTemporaryErrors(amount)) => {
                Err(RVocError::DatabaseTransactionRetryLimitReached { limit: amount })
            }
        }
        .map_err(typed_session::Error::SessionStoreConnector)
    }

    async fn read_session(
        &mut self,
        id: &SessionId,
    ) -> Result<Option<Session<RVocSessionData>>, typed_session::Error<Self::Error>> {
        todo!()
    }

    async fn update_session(
        &mut self,
        current_id: &SessionId,
        previous_id: &SessionId,
        expiry: &SessionExpiry,
        data: &RVocSessionData,
    ) -> Result<WriteSessionResult, typed_session::Error<Self::Error>> {
        todo!()
    }

    async fn delete_session(
        &mut self,
        id: &SessionId,
    ) -> Result<(), typed_session::Error<Self::Error>> {
        todo!()
    }

    async fn clear(&mut self) -> Result<(), typed_session::Error<Self::Error>> {
        todo!()
    }
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::database::schema::sessions)]
#[diesel(primary_key(id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct RVocSessionInsertable<'a> {
    id: &'a [u8],
    expiry: DateTime<Utc>,
    username: Option<&'a str>,
}

impl<'a> RVocSessionInsertable<'a> {
    fn new(id: &'a SessionId, expiry: &'a SessionExpiry, data: &'a RVocSessionData) -> Self {
        Self {
            id: id.as_ref(),
            expiry: match expiry {
                SessionExpiry::DateTime(expiry) => *expiry,
                SessionExpiry::Never => DateTime::<Utc>::MAX_UTC,
            },
            username: match data {
                RVocSessionData::Anonymous => None,
                RVocSessionData::LoggedIn(username) => Some(username.as_ref()),
            },
        }
    }
}

#[derive(Debug, Error)]
enum TryInsertSessionError {
    #[error("permanent transaction error: {0}")]
    Permanent(BoxDynError),
    #[error("too many temporary transaction errors: {0}")]
    TooManyTemporaryErrors(u64),
    #[error("session id exists")]
    SessionIdExists,
}

impl PermanentTransactionError for TryInsertSessionError {
    fn too_many_temporary_errors(limit: u64) -> Self {
        Self::TooManyTemporaryErrors(limit)
    }

    fn permanent_error(source: crate::error::BoxDynError) -> Self {
        Self::Permanent(source)
    }
}
