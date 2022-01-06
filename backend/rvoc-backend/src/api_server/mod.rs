use crate::configuration::Configuration;
use crate::database::model::users::{Session, SessionId, User};
use crate::database::model::vocabulary::Language;
use crate::error::{RVocError, RVocResult};
use cookie::{CookieBuilder, Expiration, SameSite};
use futures::StreamExt;
use futures::TryStreamExt;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::fmt::Debug;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::SystemTime;
use warp::http::header::SET_COOKIE;
use warp::http::{HeaderValue, StatusCode};
use warp::reject::Reject;
use warp::reply::Response;
use warp::{Filter, Rejection, Reply};
use wither::bson::doc;
use wither::mongodb::Database;
use wither::Model;

static SESSION_COOKIE_NAME: &str = "__Secure-sid";

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ApiCommand {
    AddLanguage {
        name: String,
    },

    ListLanguages {
        limit: usize,
    },

    /// Checks if the client is logged in.
    /// Either returns [ApiResponseData::Ok](ApiResponseData::Ok), or an error indicating missing authentication.
    IsLoggedIn,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct LoginCommand {
    pub login_name: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct SignupCommand {
    pub login_name: String,
    pub password: String,
    pub email: String,
}

struct ApiResponse {
    pub data: ApiResponseData,
    pub session: Session,
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
#[serde(tag = "response_type", content = "data", rename_all = "snake_case")]
pub enum ApiResponseData {
    Ok,
    Error(String),
    ListLanguages(Vec<Language>),
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "response_type", content = "data", rename_all = "snake_case")]
pub enum LoginResponse {
    Ok { session: Session },
    Error,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "response_type", content = "data", rename_all = "snake_case")]
pub enum SignupResponse {
    Ok,
    Error,
}

pub async fn run_api_server(configuration: &Configuration, database: &Database) -> RVocResult<()> {
    // Build command filter chain.
    let cloned_configuration = configuration.clone();
    let cloned_database = database.clone();
    let api_command = warp::post()
        // Requires the exact path "/api/command"
        .and(warp::path!("api" / "command"))
        .and(warp::body::content_length_limit(16 * 1024))
        .and(warp::any().map(move || cloned_configuration.clone()))
        .and(warp::any().map(move || cloned_database.clone()))
        .and(warp::cookie::optional(SESSION_COOKIE_NAME))
        .and_then(check_authentication)
        .untuple_one()
        .and(warp::body::json())
        .and_then(execute_api_command);

    // Build login filter chain.
    let cloned_configuration = configuration.clone();
    let cloned_database = database.clone();
    let api_login = warp::post()
        .and(warp::path!("api" / "login"))
        .and(warp::body::content_length_limit(16 * 1024))
        .and(warp::any().map(move || cloned_configuration.clone()))
        .and(warp::any().map(move || cloned_database.clone()))
        .and(warp::cookie::optional(SESSION_COOKIE_NAME))
        .and(warp::body::json())
        .and_then(execute_login);

    // Build signup filter chain.
    let cloned_configuration = configuration.clone();
    let cloned_database = database.clone();
    let api_signup = warp::post()
        .and(warp::path!("api" / "signup"))
        .and(warp::body::content_length_limit(16 * 1024))
        .and(warp::any().map(move || cloned_configuration.clone()))
        .and(warp::any().map(move || cloned_database.clone()))
        .and(warp::body::json())
        .and_then(execute_signup);

    let api_filter = api_command
        .or(api_login)
        .or(api_signup)
        .recover(handle_rejection);

    info!("Starting to serve API");
    warp::serve(api_filter)
        .run((
            Ipv4Addr::from_str(&configuration.api_listen_address)?,
            configuration.api_listen_port,
        ))
        .await;
    info!("API serving stopped");
    Ok(())
}

impl Reject for RVocError {}

async fn check_authentication(
    configuration: Configuration,
    database: Database,
    session_id: Option<String>,
) -> Result<(Configuration, Database, Session, User), Rejection> {
    match session_id {
        Some(session_id) => {
            let session_id = SessionId::try_from_string(session_id, &configuration)?;
            User::find_by_session_id(&database, &session_id)
                .await
                .map(|(user, session)| (configuration, database, session, user))
                .map_err(|_| warp::reject::custom(RVocError::NotAuthenticated))
        }
        None => Err(warp::reject::custom(RVocError::NotAuthenticated)),
    }
}

async fn execute_api_command(
    configuration: Configuration,
    database: Database,
    session: Session,
    user: User,
    api_command: ApiCommand,
) -> Result<impl Reply, Infallible> {
    let (user, session) = match user
        .update_session(session.clone(), &database, &configuration)
        .await
    {
        Ok((user, session)) => (user, session),
        Err(error) => {
            return Ok(ApiResponse {
                data: ApiResponseData::error(error),
                session,
            })
        }
    };
    Ok(api_command
        .execute(user, session.clone(), configuration, database)
        .await
        .map(|api_response_data| ApiResponse {
            data: api_response_data,
            session: session.clone(),
        })
        // This is not good and should be changed, as it leaks internal information via error messages.
        .unwrap_or_else(|error| ApiResponse {
            data: ApiResponseData::error(error),
            session,
        }))
}

async fn execute_login(
    configuration: Configuration,
    database: Database,
    session_id: Option<String>,
    login_command: LoginCommand,
) -> Result<impl Reply, Infallible> {
    // Check if user is already logged in.
    let session_id = match session_id {
        Some(session_id) => match SessionId::try_from_string(session_id.clone(), &configuration) {
            Ok(session_id) => match User::find_by_session_id(&database, &session_id).await {
                Ok((user, session)) => {
                    if user.login_name == login_command.login_name
                        && user
                            .password
                            .check(&login_command.password, &configuration)
                            .await
                            .unwrap_or(false)
                    {
                        return Ok(LoginResponse::Ok { session });
                    } else {
                        return Ok(LoginResponse::Error);
                    }
                }
                Err(_) => None,
            },
            Err(_) => None,
        },
        None => None,
    };

    // Try to log in user if not yet logged in.
    let session = match session_id {
        None => {
            if let Ok(user) = User::find_by_login_name(&database, login_command.login_name).await {
                if user
                    .password
                    .check(&login_command.password, &configuration)
                    .await
                    .unwrap_or(false)
                {
                    if let Ok((_user, session_id)) =
                        user.create_session(&database, &configuration).await
                    {
                        Some(session_id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        session_id => session_id,
    };

    if let Some(session) = session {
        Ok(LoginResponse::Ok { session })
    } else {
        Ok(LoginResponse::Error)
    }
}

async fn execute_signup(
    configuration: Configuration,
    database: Database,
    signup_command: SignupCommand,
) -> Result<impl Reply, Infallible> {
    match User::create(&signup_command, &database, &configuration).await {
        Ok(_) => Ok(SignupResponse::Ok),
        Err(error) => {
            println!("signup error:\n{:#?}", error);
            Ok(SignupResponse::Error)
        }
    }
}

async fn handle_rejection(error: Rejection) -> Result<impl Reply, Infallible> {
    debug!("{:#?}", error);

    if let Some(RVocError::NotAuthenticated) = error.find::<RVocError>() {
        Ok(warp::reply::with_status(
            "Not logged in".to_string(),
            StatusCode::FORBIDDEN,
        ))
    } else {
        // This is not good and should be changed, as it leaks internal information via error messages.
        Ok(warp::reply::with_status(
            format!("Internal server error:\n{:#?}", error),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}

impl ApiCommand {
    async fn execute(
        self,
        _user: User,
        _session: Session,
        _configuration: Configuration,
        database: Database,
    ) -> RVocResult<ApiResponseData> {
        match self {
            ApiCommand::AddLanguage { name } => {
                let mut language = Language { id: None, name };
                language.save(&database, None).await?;

                Ok(ApiResponseData::Ok)
            }
            ApiCommand::ListLanguages { limit } => {
                let limit = limit.clamp(0, 10_000);
                let language_cursor = Language::find(&database, None, None).await?;
                Ok(ApiResponseData::ListLanguages(
                    language_cursor.take(limit).try_collect().await?,
                ))
            }
            ApiCommand::IsLoggedIn => Ok(ApiResponseData::Ok),
        }
    }
}

impl ApiResponseData {
    pub fn error(error: RVocError) -> Self {
        Self::Error(format!("{:?}", error))
    }

    #[allow(unused)]
    pub fn is_error(&self) -> bool {
        matches!(self, ApiResponseData::Error(_))
    }
}

fn create_session_cookie_header_value(session: &Session) -> HeaderValue {
    let cookie = CookieBuilder::new(SESSION_COOKIE_NAME, session.session_id().to_string())
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Strict)
        .expires(Expiration::DateTime(
            SystemTime::from(session.expires().to_chrono()).into(),
        ))
        .finish();
    HeaderValue::try_from(cookie.to_string()).expect("invalid cookie string")
}

impl Reply for ApiResponse {
    fn into_response(self) -> Response {
        let mut response = warp::reply::json(&self.data).into_response();

        response.headers_mut().append(
            SET_COOKIE,
            create_session_cookie_header_value(&self.session),
        );
        response
    }
}

impl Reply for LoginResponse {
    fn into_response(self) -> warp::reply::Response {
        match self {
            LoginResponse::Ok { session } => {
                let mut response = warp::reply::json(&ApiResponseData::Ok).into_response();
                let cookie =
                    CookieBuilder::new(SESSION_COOKIE_NAME, session.session_id().to_string())
                        .secure(true)
                        .http_only(true)
                        .same_site(SameSite::Strict)
                        .expires(Expiration::DateTime(
                            SystemTime::from(session.expires().to_chrono()).into(),
                        ))
                        .finish();
                response.headers_mut().append(
                    SET_COOKIE,
                    HeaderValue::try_from(cookie.to_string()).expect("invalid cookie string"),
                );
                response
            }
            LoginResponse::Error => {
                warp::reply::json(&ApiResponseData::Error("Authentication error".to_string()))
                    .into_response()
            }
        }
    }
}

impl Reply for SignupResponse {
    fn into_response(self) -> Response {
        warp::reply::json(&match self {
            SignupResponse::Ok => ApiResponseData::Ok,
            SignupResponse::Error => ApiResponseData::Error("Signup error".to_string()),
        })
        .into_response()
    }
}
