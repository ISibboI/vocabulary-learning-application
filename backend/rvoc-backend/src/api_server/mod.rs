use crate::configuration::Configuration;
use crate::database::model::vocabulary::Language;
use crate::error::{RVocError, RVocResult};
use futures::StreamExt;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::Ipv4Addr;
use std::str::FromStr;
use warp::{Filter, Reply};
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

pub async fn run_api_server(configuration: &Configuration, database: Database) -> RVocResult<()> {
    let api_command = warp::post()
        .and(warp::path("command"))
        .and(warp::body::content_length_limit(16 * 1024))
        .and(warp::body::json())
        .and(warp::any().map(move || database.clone()))
        .and_then(ApiCommand::execute_consume);

    warp::serve(api_command)
        .run((
            Ipv4Addr::from_str(&configuration.api_listen_address)?,
            configuration.api_listen_port,
        ))
        .await;
    Ok(())
}

impl ApiCommand {
    pub async fn execute_consume(self, database: Database) -> Result<impl Reply, Infallible> {
        Ok(self
            .execute_internal(database)
            .await
            .map(|api_response| warp::reply::json(&api_response))
            .unwrap_or_else(|error| warp::reply::json(&ApiResponse::error(error))))
    }

    async fn execute_internal(self, database: Database) -> RVocResult<ApiResponse> {
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
