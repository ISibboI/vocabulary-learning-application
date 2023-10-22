use api_commands::Login;
use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json,
};
use diesel::QueryDsl;
use tracing::{info, instrument};
use typed_session_axum::{SessionHandle, WritableSession};

use crate::{
    error::{RVocError, RVocResult, UserError},
    web::user::password_hash::PasswordHash,
};

use super::{
    session::RVocSessionData, user::model::Username, WebConfiguration, WebDatabaseConnectionPool,
};

pub async fn ensure_logged_in<B>(mut request: Request<B>, next: Next<B>) -> Response {
    let session: &SessionHandle<RVocSessionData> = request.extensions().get().unwrap();
    let session = session.read().await;
    let session_data = session.data();

    match session_data {
        RVocSessionData::Anonymous => return StatusCode::UNAUTHORIZED.into_response(),
        RVocSessionData::LoggedIn(username) => {
            let username = username.clone();
            drop(session);
            request.extensions_mut().insert(LoggedInUser(username));
        }
    }

    next.run(request).await
}

#[instrument(err, skip(database_connection_pool, configuration))]
pub async fn login(
    Extension(database_connection_pool): WebDatabaseConnectionPool,
    Extension(configuration): WebConfiguration,
    mut session: WritableSession<RVocSessionData>,
    Json(login): Json<Login>,
) -> RVocResult<StatusCode> {
    // any failed login attempt should cause a logout
    *session.data_mut() = RVocSessionData::Anonymous;

    configuration.verify_username_length(&login.name)?;
    configuration.verify_password_length(&login.password)?;

    database_connection_pool
        .execute_transaction::<_, RVocError>(
            |database_connection| {
                Box::pin(async {
                    use crate::database::schema::users;
                    use diesel::ExpressionMethods;
                    use diesel::OptionalExtension;
                    use diesel_async::RunQueryDsl;

                    let configuration = configuration.clone();

                    // get password hash
                    let password_hash: String = if let Some(password_hash) = users::table
                        .select(users::password_hash)
                        .filter(users::name.eq(&login.name))
                        .first(database_connection)
                        .await
                        .optional()?
                    {
                        if let Some(password_hash) = password_hash {
                            password_hash
                        } else {
                            // Here the optional() returned a row, but with a null password hash.
                            info!("User has no password: {:?}", login.name);
                            return Err(UserError::InvalidUsernamePassword.into());
                        }
                    } else {
                        // Here the optional() returned None, i.e. no row was found.
                        info!("User not found: {:?}", login.name);
                        return Err(UserError::InvalidUsernamePassword.into());
                    };

                    // verify password hash
                    let mut password_hash = PasswordHash::from(password_hash);
                    let verify_result =
                        password_hash.verify(login.password.clone(), configuration)?;

                    if !verify_result.matches {
                        info!("Wrong password for user: {:?}", login.name);
                        return Err(UserError::InvalidUsernamePassword.into());
                    }

                    // update password hash if modified
                    if verify_result.modified {
                        let affected_rows = diesel::update(users::table)
                            .filter(users::name.eq(&login.name))
                            .set(users::password_hash.eq(Option::<String>::from(password_hash)))
                            .execute(database_connection)
                            .await?;

                        if affected_rows != 1 {
                            unreachable!(
                                "Updated exactly one existing row, but {affected_rows} were affected"
                            );
                        }
                    }

                    Ok(())
                })
            },
            configuration.maximum_transaction_retry_count,
        )
        .await
        .map_err(|error| match error {
            error @ RVocError::UserError(_) => error,
            error => RVocError::Login {
                source: Box::new(error),
            },
        })?;

    *session.data_mut() = RVocSessionData::LoggedIn(Username::new(login.name.clone()));

    Ok(StatusCode::NO_CONTENT)
}

#[instrument(err)]
pub async fn logout(mut session: WritableSession<RVocSessionData>) -> RVocResult<StatusCode> {
    session.delete();

    Ok(StatusCode::NO_CONTENT)
}

/// If this extension is found, it means that the request was made by the contained username.
#[derive(Debug, Clone)]
pub struct LoggedInUser(Username);

impl From<LoggedInUser> for String {
    fn from(value: LoggedInUser) -> Self {
        value.0.into()
    }
}

impl AsRef<str> for LoggedInUser {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
