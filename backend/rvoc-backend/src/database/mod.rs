use crate::configuration::Configuration;
use crate::database::model::sync_model;
use crate::error::{RVocError, RVocResult};
use log::{info, warn};
use std::time::Duration;
use wither::bson::doc;
use wither::mongodb::options::{ClientOptions, Credential, ServerAddress};
use wither::mongodb::{Client, Database};

pub mod model;

pub async fn connect_to_database(configuration: Configuration) -> RVocResult<Database> {
    info!(
        "Initialising mongodb driver for database '{}' at '{}'...",
        configuration.mongodb_database, configuration.mongodb_uri
    );
    let client = Client::with_options(
        ClientOptions::builder()
            .hosts(vec![ServerAddress::Tcp {
                host: configuration.mongodb_host.clone(),
                port: Some(configuration.mongodb_port),
            }])
            .credential(
                Credential::builder()
                    .username(configuration.mongodb_user.clone())
                    .password(configuration.mongodb_password.clone())
                    .build(),
            )
            .connect_timeout(Duration::from_secs(configuration.mongodb_connect_timeout))
            .build(),
    )
    .map_err(RVocError::CouldNotSetUpDatabaseClient)?;
    let database = client.database(&configuration.mongodb_database);

    info!("Checking if database is available...");
    while let Err(error) = database.run_command(doc! {"ping": 1}, None).await {
        warn!("Waiting for database to become available: {:?}", error);
    }
    info!("Database available");

    sync_model(&database).await?;

    Ok(database)
}
