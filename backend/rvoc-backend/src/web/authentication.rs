use api_commands::Login;
use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json,
};
use chrono::Utc;
use tracing::{info, instrument};
use typed_session_axum::{SessionHandle, WritableSession};

use crate::{
    error::{RVocError, RVocResult, UserError},
    model::user::{username::Username, UserLoginInfo},
};

use super::{session::RVocSessionData, WebConfiguration, WebDatabaseConnectionPool};

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

    let Login { username, password } = login;
    let username = Username::new(username, &configuration)?;

    database_connection_pool
        .execute_transaction::<_, RVocError>(
            |database_connection| {
                Box::pin(async {
                    use crate::database::schema::users;
                    use diesel::ExpressionMethods;
                    use diesel::OptionalExtension;
                    use diesel::{QueryDsl, SelectableHelper};
                    use diesel_async::RunQueryDsl;

                    let configuration = configuration.clone();
                    let now = Utc::now();

                    // get user login info
                    let Some(mut user_login_info) = users::table
                        .select(UserLoginInfo::as_select())
                        .filter(users::name.eq(username.as_ref()))
                        .first(database_connection)
                        .await
                        .optional()?
                    else {
                        // Here the optional() returned None, i.e. no row was found.
                        info!("User not found: {:?}", username);
                        return Err(UserError::InvalidUsernamePassword.into());
                    };

                    // check and update rate limit
                    if !user_login_info.try_login_attempt(now, configuration.as_ref()) {
                        // The user's login rate limit was reached.
                        info!("User login rate limit reached: {:?}", username);
                        return Err(UserError::UserLoginRateLimitReached.into());
                    }

                    // verify password hash
                    let verify_result = user_login_info
                        .password_hash
                        .verify(password.clone(), configuration)?;

                    if !verify_result.matches {
                        info!("Wrong password for user: {:?}", username);
                        return Err(UserError::InvalidUsernamePassword.into());
                    }

                    // update login info
                    let username = user_login_info.name.clone();
                    let affected_rows = diesel::update(users::table)
                        .set(user_login_info)
                        .filter(users::name.eq(username.as_ref()))
                        .execute(database_connection)
                        .await?;

                    if affected_rows != 1 {
                        unreachable!(
                            "Updated exactly one existing row, but {affected_rows} were affected"
                        );
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

    *session.data_mut() = RVocSessionData::LoggedIn(username);

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
