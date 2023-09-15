use api_commands::CreateAccount;
use axum::{http::StatusCode, Extension, Json};
use tracing::instrument;
use typed_session_axum::WritableSession;

use crate::error::{RVocError, RVocResult, UserError};

use self::{
    model::{User, Username},
    password_hash::PasswordHash,
};

use super::{
    authentication::LoggedInUser, session::RVocSessionData, WebConfiguration,
    WebDatabaseConnectionPool,
};

pub mod model;
pub mod password_hash;

#[instrument(err, skip(database_connection_pool, configuration))]
pub async fn create_account(
    Extension(database_connection_pool): WebDatabaseConnectionPool,
    Extension(configuration): WebConfiguration,
    Json(create_account): Json<CreateAccount>,
) -> RVocResult<StatusCode> {
    configuration.verify_username_length(&create_account.name)?;
    configuration.verify_password_length(&create_account.password)?;

    let user = User {
        name: Username::new(create_account.name),
        password_hash: PasswordHash::new(create_account.password, configuration)?,
    };

    database_connection_pool
        .execute_transaction_without_retries::<_, RVocError>(|database_connection| {
            Box::pin(async {
                use crate::database::schema::users::dsl::*;
                use diesel_async::RunQueryDsl;

                let username = user.name.clone().into();
                match diesel::insert_into(users)
                    .values(user)
                    .execute(database_connection)
                    .await
                {
                    Ok(1) => Ok(StatusCode::CREATED),
                    Ok(affected_rows) => {
                        unreachable!("inserting exactly one row, but affected {affected_rows} rows")
                    }
                    Err(diesel::result::Error::DatabaseError(
                        diesel::result::DatabaseErrorKind::UniqueViolation,
                        _,
                    )) => Err(RVocError::UserError(
                        crate::error::UserError::UsernameExists { username },
                    )),
                    Err(error) => Err(RVocError::CreateUser {
                        source: Box::new(error),
                    }),
                }
            })
        })
        .await
}

#[instrument(err, skip(database_connection_pool))]
pub async fn delete_account(
    Extension(username): Extension<LoggedInUser>,
    Extension(database_connection_pool): WebDatabaseConnectionPool,
    mut session: WritableSession<RVocSessionData>,
) -> RVocResult<StatusCode> {
    session.delete();

    database_connection_pool
        .execute_transaction_without_retries(|database_connection| {
            Box::pin(async {
                use crate::database::schema::sessions;
                use crate::database::schema::users;
                use diesel::ExpressionMethods;
                use diesel_async::RunQueryDsl;

                diesel::delete(sessions::table)
                    .filter(sessions::username.eq(username.as_ref()))
                    .execute(database_connection)
                    .await
                    .map_err(|error| RVocError::DeleteAllUserSessions {
                        source: Box::new(error),
                    })?;

                match diesel::delete(users::table)
                    .filter(users::name.eq(username.as_ref()))
                    .execute(database_connection)
                    .await
                {
                    Ok(0) => Err(UserError::UsernameDoesNotExist {
                        username: username.into(),
                    }
                    .into()),
                    Ok(1) => Ok(StatusCode::NO_CONTENT),
                    Ok(affected_rows) => {
                        unreachable!("deleted exactly one user, but affected {affected_rows} rows")
                    }
                    Err(error) => Err(RVocError::DeleteUser {
                        source: Box::new(error),
                    }),
                }
            })
        })
        .await
}
