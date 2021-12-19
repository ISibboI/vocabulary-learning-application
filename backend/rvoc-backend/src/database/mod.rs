use crate::configuration::Configuration;
use crate::database::model::sync_model;
use crate::error::RVocResult;
use log::info;
use wither::mongodb::options::{ClientOptions, Credential, ServerAddress};
use wither::mongodb::{Client, Database};

pub mod model;

pub async fn connect_to_database(configuration: &Configuration) -> RVocResult<Database> {
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
            .build(),
    )?;
    let database = client.database(&configuration.mongodb_database);
    info!("Driver ready");
    sync_model(&database).await?;

    Ok(database)
}
