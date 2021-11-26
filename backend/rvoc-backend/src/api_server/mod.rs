use crate::error::RVocResult;
use crate::Configuration;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::str::FromStr;
use warp::{Filter, Rejection, Reply};
use wither::mongodb::Database;

#[derive(Deserialize, Serialize)]
pub enum ApiCommand {}

#[derive(Deserialize, Serialize)]
pub struct ApiResponse {
    error: Option<String>,
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
    pub async fn execute_consume(self, database: Database) -> Result<impl Reply, Rejection> {
        self.execute(&database)
            .await
            .map(|api_response| warp::reply::json(&api_response))
    }

    pub async fn execute(&self, database: &Database) -> Result<ApiResponse, Rejection> {
        Ok(ApiResponse::ok())
    }
}

impl ApiResponse {
    pub fn ok() -> Self {
        Self { error: None }
    }
}
