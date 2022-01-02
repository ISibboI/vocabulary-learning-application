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
use warp::http::StatusCode;
use warp::reject::Reject;
use warp::{Filter, Rejection, Reply};
use wither::bson::doc;
use wither::mongodb::Database;
use wither::Model;

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ApiCommand {
    AddLanguage { name: String },

    ListLanguages { limit: usize },
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

pub async fn run_api_server(configuration: Configuration, database: Database) -> RVocResult<()> {
    let moved_configuration = configuration.clone();
    let api_command = warp::post()
        // Requires the exact path "/api/command"
        .and(warp::path!("api" / "command"))
        .and(warp::body::content_length_limit(16 * 1024))
        .and(warp::any().map(move || moved_configuration.clone()))
        .and(warp::any().map(move || database.clone()))
        .and(warp::cookie::optional("sid"))
        .and_then(check_authentication)
        .untuple_one()
        .and(warp::body::json())
        .and_then(execute_api_command)
        .recover(handle_rejection);

    info!("Starting to serve API");
    warp::serve(api_command)
        .run((
            Ipv4Addr::from_str(&configuration.api_listen_address)?,
            configuration.api_listen_port,
        ))
        .await;
    info!("API serving stopped");
    Ok(())
}

#[derive(Debug)]
struct AuthenticationRejection;
impl Reject for AuthenticationRejection {}
impl Reject for RVocError {}

async fn check_authentication(
    configuration: Configuration,
    database: Database,
    session_id: Option<String>,
) -> Result<(Configuration, Database, User), Rejection> {
    match session_id {
        Some(session_id) => User::find_by_session_id(
            &database,
            &SessionId::try_from_string(session_id, configuration.clone())?,
        )
        .await
        .map(|user| (configuration, database, user))
        .map_err(|_| warp::reject::custom(AuthenticationRejection)),
        None => Err(warp::reject::custom(AuthenticationRejection)),
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

async fn handle_rejection(error: Rejection) -> Result<impl Reply, Infallible> {
    if error.find::<AuthenticationRejection>().is_some() {
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
