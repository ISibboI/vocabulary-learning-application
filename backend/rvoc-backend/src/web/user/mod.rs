use api_commands::CreateAccount;
use axum::{http::StatusCode, Extension, Json};

use crate::error::{RVocError, RVocResult};

use self::{
    model::{User, Username},
    password_hash::PasswordHash,
};

use super::{LoggedInUser, WebConfiguration, WebDatabaseConnectionPool};

pub mod model;
pub mod password_hash;

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
        .execute_transaction::<_, RVocError>(|database_connection| {
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
                        database_error_information,
                    )) => {
                        if database_error_information.table_name() == Some("users")
                            && database_error_information.column_name() == Some("name")
                        {
                            Err(RVocError::UserError(
                                crate::error::UserError::UsernameExists { username },
                            ))
                        } else {
                            Err(RVocError::CreateUser {
                                source: Box::new(diesel::result::Error::DatabaseError(
                                    diesel::result::DatabaseErrorKind::UniqueViolation,
                                    database_error_information,
                                )),
                            })
                        }
                    }
                    Err(error) => Err(RVocError::CreateUser {
                        source: Box::new(error),
                    }),
                }
            })
        })
        .await
}

pub async fn delete_account(
    Extension(username): Extension<LoggedInUser>,
    Extension(database_connection_pool): WebDatabaseConnectionPool,
) -> RVocResult<StatusCode> {
    database_connection_pool
        .execute_transaction(|database_connection| {
            Box::pin(async {
                use crate::database::schema::users::dsl::*;
                use diesel::ExpressionMethods;
                use diesel_async::RunQueryDsl;

                match diesel::delete(users)
                    .filter(name.eq(username.0.as_ref()))
                    .execute(database_connection)
                    .await
                {
                    Ok(0) => Err(RVocError::UserError(
                        crate::error::UserError::UsernameDoesNotExist {
                            username: username.into(),
                        },
                    )),
                    Ok(1) => Ok(StatusCode::OK),
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
