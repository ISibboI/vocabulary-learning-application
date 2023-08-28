use api_commands::Login;
use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json,
};
use diesel::QueryDsl;
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

pub async fn login(
    Extension(database_connection_pool): WebDatabaseConnectionPool,
    Extension(configuration): WebConfiguration,
    mut session: WritableSession<RVocSessionData>,
    Json(login): Json<Login>,
) -> RVocResult<StatusCode> {
    configuration.verify_username_length(&login.name)?;
    configuration.verify_password_length(&login.password)?;

    database_connection_pool
        .execute_transaction(|database_connection| {
            Box::pin(async {
                use crate::database::schema::users;
                use diesel::ExpressionMethods;
                use diesel::OptionalExtension;
                use diesel_async::RunQueryDsl;

                // get password hash
                let password_hash: String = if let Some(password_hash) = users::table
                    .select(users::password_hash)
                    .filter(users::name.eq(&login.name))
                    .first(database_connection)
                    .await
                    .optional()
                    .map_err(|error| RVocError::Login {
                        source: Box::new(error),
                    })? {
                    password_hash
                } else {
                    return Err(UserError::InvalidUsernamePassword.into());
                };

                // verify password hash
                let mut password_hash = PasswordHash::from(password_hash);
                let verify_result = password_hash.verify(login.password, configuration)?;

                if !verify_result.matches {
                    *session.data_mut() = RVocSessionData::Anonymous;
                    return Err(UserError::InvalidUsernamePassword.into());
                }

                // update password hash if modified
                if verify_result.modified {
                    let affected_rows = diesel::update(users::table)
                        .filter(users::name.eq(&login.name))
                        .set(users::password_hash.eq(String::from(password_hash)))
                        .execute(database_connection)
                        .await
                        .map_err(|error| RVocError::Login {
                            source: Box::new(error),
                        })?;

                    if affected_rows != 1 {
                        unreachable!(
                            "Updated exactly one existing row, but {affected_rows} were affected"
                        );
                    }
                }

                *session.data_mut() = RVocSessionData::LoggedIn(Username::new(login.name));

                Ok(StatusCode::NO_CONTENT)
            })
        })
        .await
}

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
