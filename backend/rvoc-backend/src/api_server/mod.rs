use crate::configuration::Configuration;
use crate::database::model::users::{SessionId, User};
use crate::database::model::vocabulary::Language;
use crate::error::{RVocError, RVocResult};
use futures::StreamExt;
use futures::TryStreamExt;
use log::info;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::fmt::Debug;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;
use warp::http::header::SET_COOKIE;
use warp::http::{HeaderValue, StatusCode};
use warp::reject::Reject;
use warp::{Filter, Rejection, Reply};
use wither::bson::doc;
use wither::mongodb::Database;
use wither::Model;

static SESSION_COOKIE_NAME: &str = "__Secure-sid";

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ApiCommand {
    AddLanguage { name: String },

    ListLanguages { limit: usize },
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct LoginCommand {
    login_name: String,
    password: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ApiResponse {
    pub error: Option<String>,
    #[serde(flatten)]
    pub data: ApiResponseData,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "response_type", content = "data", rename_all = "snake_case")]
pub enum ApiResponseData {
    None,
    ListLanguages(Vec<Language>),
}

pub enum LoginResponse {
    Ok {
        session_id: SessionId,
        max_age: Duration,
    },
    Error,
}

impl Reply for LoginResponse {
    fn into_response(self) -> warp::reply::Response {
        match self {
            LoginResponse::Ok {
                session_id,
                max_age,
            } => {
                let mut response = warp::reply::json(&ApiResponse {
                    error: None,
                    data: ApiResponseData::None,
                })
                .into_response();
                response.headers_mut().append(
                    SET_COOKIE,
                    HeaderValue::try_from(format!(
                        "{}={}; Secure; HttpOnly; SameSite=Strict; Max-Age={}",
                        SESSION_COOKIE_NAME,
                        session_id.to_string(),
                        max_age.as_secs(),
                    ))
                    .expect("invalid cookie string"),
                );
                response
            }
            LoginResponse::Error => warp::reply::json(&ApiResponse {
                error: Some("Authentication error".to_string()),
                data: ApiResponseData::None,
            })
            .into_response(),
        }
    }
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

    let api_filter = api_command.or(api_login).recover(handle_rejection);

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
) -> Result<(Configuration, Database, User), Rejection> {
    match session_id {
        Some(session_id) => User::find_by_session_id(
            &database,
            &SessionId::try_from_string(session_id, &configuration)?,
        )
        .await
        .map(|user| (configuration, database, user))
        .map_err(|_| warp::reject::custom(RVocError::NotAuthenticated)),
        None => Err(warp::reject::custom(RVocError::NotAuthenticated)),
    }
}

async fn execute_api_command(
    _configuration: Configuration,
    database: Database,
    _user: User,
    api_command: ApiCommand,
) -> Result<impl Reply, Infallible> {
    Ok(api_command
        .execute(database)
        .await
        .map(|api_response| warp::reply::json(&api_response))
        // This is not good and should be changed, as it leaks internal information via error messages.
        .unwrap_or_else(|error| warp::reply::json(&ApiResponse::error(error))))
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
                Ok(user) => {
                    if user.login_name == login_command.login_name
                        && user
                            .password
                            .check(&login_command.password, &configuration)
                            .await
                            .unwrap_or(false)
                    {
                        return Ok(LoginResponse::Ok {
                            session_id,
                            max_age: Duration::from_secs(
                                configuration.session_cookie_max_age_seconds,
                            ),
                        });
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
    let session_id = match session_id {
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

    if let Some(session_id) = session_id {
        Ok(LoginResponse::Ok {
            session_id,
            max_age: Duration::from_secs(configuration.session_cookie_max_age_seconds),
        })
    } else {
        Ok(LoginResponse::Error)
    }
}

async fn handle_rejection(error: Rejection) -> Result<impl Reply, Infallible> {
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
    async fn execute(self, database: Database) -> RVocResult<ApiResponse> {
        match self {
            ApiCommand::AddLanguage { name } => {
                let mut language = Language { id: None, name };
                language.save(&database, None).await?;

                Ok(ApiResponse::ok())
            }
            ApiCommand::ListLanguages { limit } => {
                let limit = limit.clamp(0, 10_000);
                let language_cursor = Language::find(&database, None, None).await?;
                Ok(ApiResponse::ok_with_data(ApiResponseData::ListLanguages(
                    language_cursor.take(limit).try_collect().await?,
                )))
            }
        }
    }
}

impl ApiResponse {
    pub fn ok() -> Self {
        Self {
            error: None,
            data: ApiResponseData::None,
        }
    }

    pub fn ok_with_data(data: ApiResponseData) -> Self {
        Self { error: None, data }
    }

    pub fn error(error: RVocError) -> Self {
        Self {
            error: Some(format!("{:?}", error)),
            data: ApiResponseData::None,
        }
    }
}
