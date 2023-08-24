use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use thiserror::Error;
use tracing::debug;
use typed_session::{Session, SessionExpiry, SessionId, WriteSessionResult};
use typed_session_axum::typed_session::SessionStoreConnector;

use crate::{
    configuration::Configuration,
    database::{transactions::PermanentTransactionError, RVocAsyncDatabaseConnectionPool},
    error::{BoxDynError, RVocError},
};

use super::user::model::Username;

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
            .execute_transaction::<_, TryInsertSessionError>(|database_connection| {
                Box::pin(async {
                    use crate::database::schema::sessions::dsl::*;
                    use diesel_async::RunQueryDsl;

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
                                    TryInsertSessionError::Error(
                                        diesel::result::Error::DatabaseError(
                                            diesel::result::DatabaseErrorKind::UniqueViolation,
                                            database_error_information,
                                        )
                                        .into(),
                                    )
                                }
                            }
                            error => TryInsertSessionError::Error(error.into()),
                        })?;

                    Ok(())
                })
            })
            .await
        {
            Ok(()) => Ok(WriteSessionResult::Ok(())),
            Err(TryInsertSessionError::SessionIdExists) => Ok(WriteSessionResult::SessionIdExists),
            Err(TryInsertSessionError::Error(error)) => {
                Err(RVocError::InsertSession { source: error })
            }
            Err(TryInsertSessionError::PreviousSessionIdDoesNotExist) => unreachable!(),
        }
        .map_err(typed_session::Error::SessionStoreConnector)
    }

    async fn read_session(
        &mut self,
        session_id: SessionId,
    ) -> Result<Option<Session<RVocSessionData>>, typed_session::Error<Self::Error>> {
        if let Some(queryable) = self
            .database_connection_pool
            .execute_transaction(|database_connection| {
                use crate::database::schema::sessions::dsl::*;
                use diesel::OptionalExtension;
                use diesel::QueryDsl;
                use diesel::SelectableHelper;
                use diesel_async::RunQueryDsl;

                Box::pin(async {
                    sessions
                        .find(session_id.as_ref())
                        .select(RVocSessionQueryable::as_select())
                        .first(database_connection)
                        .await
                        .optional()
                        .map_err(|error| RVocError::ReadSession {
                            source: Box::new(error),
                        })
                })
            })
            .await
            .map_err(typed_session::Error::SessionStoreConnector)?
        {
            let expiry = if queryable.expiry == DateTime::<Utc>::MAX_UTC {
                SessionExpiry::Never
            } else {
                SessionExpiry::DateTime(queryable.expiry)
            };
            let data = match queryable.username {
                Some(name) => RVocSessionData::LoggedIn(Username::new(name)),
                None => RVocSessionData::Anonymous,
            };

            Ok(Some(Session::new_from_session_store(
                session_id, expiry, data,
            )))
        } else {
            Ok(None)
        }
    }

    async fn update_session(
        &mut self,
        current_id: &SessionId,
        previous_id: &SessionId,
        session_expiry: &SessionExpiry,
        data: &RVocSessionData,
    ) -> Result<WriteSessionResult, typed_session::Error<Self::Error>> {
        match self
            .database_connection_pool
            .execute_transaction::<_, TryInsertSessionError>(|database_connection| {
                Box::pin(async {
                    use crate::database::schema::sessions::dsl::*;
                    use diesel::ExpressionMethods;
                    use diesel_async::RunQueryDsl;

                    let deleted_count: i64 = diesel::delete(sessions)
                        .filter(id.eq(previous_id.as_ref()))
                        .returning(diesel::dsl::count_star())
                        .get_result(database_connection)
                        .await
                        .map_err(|error| TryInsertSessionError::Error(Box::new(error)))?;

                    if deleted_count != 1 {
                        assert_eq!(deleted_count, 0);
                        return Err(TryInsertSessionError::PreviousSessionIdDoesNotExist);
                    }

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
                                    TryInsertSessionError::Error(
                                        diesel::result::Error::DatabaseError(
                                            diesel::result::DatabaseErrorKind::UniqueViolation,
                                            database_error_information,
                                        )
                                        .into(),
                                    )
                                }
                            }
                            error => TryInsertSessionError::Error(error.into()),
                        })?;

                    Ok(())
                })
            })
            .await
        {
            Ok(()) => Ok(WriteSessionResult::Ok(())),
            Err(TryInsertSessionError::SessionIdExists) => Ok(WriteSessionResult::SessionIdExists),
            Err(TryInsertSessionError::PreviousSessionIdDoesNotExist) => {
                Err(typed_session::Error::UpdatedSessionDoesNotExist)
            }
            Err(TryInsertSessionError::Error(error)) => {
                Err(RVocError::InsertSession { source: error })
                    .map_err(typed_session::Error::SessionStoreConnector)
            }
        }
    }

    async fn delete_session(
        &mut self,
        session_id: &SessionId,
    ) -> Result<(), typed_session::Error<Self::Error>> {
        self.database_connection_pool
            .execute_transaction(|database_connection| {
                use crate::database::schema::sessions::dsl::*;
                use diesel::ExpressionMethods;
                use diesel_async::RunQueryDsl;

                Box::pin(async {
                    let deleted_count: i64 = diesel::delete(sessions)
                        .filter(id.eq(session_id.as_ref()))
                        .returning(diesel::dsl::count_star())
                        .get_result(database_connection)
                        .await
                        .map_err(|error| RVocError::ReadSession {
                            source: Box::new(error),
                        })?;

                    if deleted_count != 1 {
                        assert_eq!(deleted_count, 0);
                        debug!("Session id that was supposed to be deleted was not found");
                    }

                    Ok(())
                })
            })
            .await
            .map_err(typed_session::Error::SessionStoreConnector)
    }

    async fn clear(&mut self) -> Result<(), typed_session::Error<Self::Error>> {
        self.database_connection_pool
            .execute_transaction(|database_connection| {
                use crate::database::schema::sessions::dsl::*;
                use diesel_async::RunQueryDsl;

                Box::pin(async {
                    diesel::delete(sessions)
                        .execute(database_connection)
                        .await
                        .map_err(|error| RVocError::ReadSession {
                            source: Box::new(error),
                        })
                })
            })
            .await
            .map(|_| ())
            .map_err(typed_session::Error::SessionStoreConnector)
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

#[derive(Selectable, Queryable, Debug)]
#[diesel(table_name = crate::database::schema::sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct RVocSessionQueryable {
    expiry: DateTime<Utc>,
    username: Option<String>,
}

#[derive(Debug, Error)]
enum TryInsertSessionError {
    #[error("permanent transaction error: {0}")]
    Error(BoxDynError),
    #[error("session id exists")]
    SessionIdExists,
    #[error("previous session id does not exist")]
    PreviousSessionIdDoesNotExist,
}

impl PermanentTransactionError for TryInsertSessionError {
    fn permanent_error(source: crate::error::BoxDynError) -> Self {
        Self::Error(source)
    }
}
