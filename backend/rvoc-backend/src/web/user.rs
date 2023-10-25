use crate::{
    error::{RVocError, RVocResult, UserError},
    model::user::{password_hash::PasswordHash, username::Username, User},
};
use api_commands::CreateAccount;
use axum::{http::StatusCode, Extension, Json};
use tracing::instrument;
use typed_session_axum::WritableSession;

use super::{
    authentication::LoggedInUser, session::RVocSessionData, WebConfiguration,
    WebDatabaseConnectionPool,
};

#[instrument(err, skip(database_connection_pool, configuration))]
pub async fn create_account(
    Extension(database_connection_pool): WebDatabaseConnectionPool,
    Extension(configuration): WebConfiguration,
    Json(create_account): Json<CreateAccount>,
) -> RVocResult<StatusCode> {
    let CreateAccount { username, password } = create_account;
    let username = Username::new(username, &configuration)?;

    let user = User {
        name: username,
        password_hash: PasswordHash::new(password, &configuration)?,
    };

    database_connection_pool
        .execute_transaction::<_, RVocError>(
            |database_connection| {
                Box::pin(async {
                    use crate::database::schema::users::dsl::*;
                    use diesel_async::RunQueryDsl;

                    let user = user.clone();
                    let username = user.name.clone().into();
                    match diesel::insert_into(users)
                        .values(user)
                        .execute(database_connection)
                        .await
                    {
                        Ok(1) => Ok(StatusCode::CREATED),
                        Ok(affected_rows) => {
                            unreachable!(
                                "inserting exactly one row, but affected {affected_rows} rows"
                            )
                        }
                        Err(diesel::result::Error::DatabaseError(
                            diesel::result::DatabaseErrorKind::UniqueViolation,
                            _,
                        )) => Err(
                            RVocError::UserError(crate::error::UserError::UsernameExists {
                                username,
                            })
                            .into(),
                        ),
                        Err(error) => Err(error.into()),
                    }
                })
            },
            configuration.maximum_transaction_retry_count,
        )
        .await
        .map_err(|error| match error {
            error @ RVocError::UserError(_) => error,
            error => RVocError::CreateUser {
                source: Box::new(error),
            },
        })
}

#[instrument(err, skip(database_connection_pool))]
pub async fn delete_account(
    Extension(username): Extension<LoggedInUser>,
    Extension(database_connection_pool): WebDatabaseConnectionPool,
    Extension(configuration): WebConfiguration,
    mut session: WritableSession<RVocSessionData>,
) -> RVocResult<StatusCode> {
    session.delete();

    database_connection_pool
        .execute_transaction(
            |database_connection| {
                Box::pin(async {
                    use crate::database::schema::sessions;
                    use crate::database::schema::users;
                    use diesel::ExpressionMethods;
                    use diesel_async::RunQueryDsl;

                    diesel::delete(sessions::table)
                        .filter(sessions::username.eq(username.as_ref()))
                        .execute(database_connection)
                        .await?;

                    match diesel::delete(users::table)
                        .filter(users::name.eq(username.as_ref()))
                        .execute(database_connection)
                        .await
                    {
                        Ok(0) => Err(UserError::UsernameDoesNotExist {
                            username: username.clone().into(),
                        }
                        .into()),
                        Ok(1) => Ok(StatusCode::NO_CONTENT),
                        Ok(affected_rows) => {
                            unreachable!(
                                "deleted exactly one user, but affected {affected_rows} rows"
                            )
                        }
                        Err(error) => Err(error.into()),
                    }
                })
            },
            configuration.maximum_transaction_retry_count,
        )
        .await
        .map_err(|error| match error {
            error @ RVocError::UserError(_) => error,
            error => RVocError::DeleteUser {
                source: Box::new(error),
            },
        })
}
