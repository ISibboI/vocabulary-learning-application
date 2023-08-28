use api_commands::CreateAccount;
use axum::{Extension, Json};

use crate::error::{RVocError, RVocResult};

use self::{
    model::{User, Username},
    password_hash::PasswordHash,
};

use super::{WebConfiguration, WebDatabaseConnectionPool};

pub mod model;
pub mod password_hash;

pub async fn create_account(
    Extension(database_connection_pool): WebDatabaseConnectionPool,
    Extension(configuration): WebConfiguration,
    Json(create_account): Json<CreateAccount>,
) -> RVocResult<()> {
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
                    Ok(1) => Ok(()),
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
