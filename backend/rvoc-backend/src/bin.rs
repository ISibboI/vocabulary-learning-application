use crate::configuration::Configuration;
use crate::error::RVocResult;
use diesel_async::{AsyncPgConnection, pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager}};
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use log::{error, info};
use tokio::runtime::Builder;

mod configuration;
mod error;

pub fn main() -> RVocResult<()> {
    let configuration = Configuration::from_environment()?;

    info!("Building tokio runtime...");
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| panic!("Cannot create tokio runtime: {:?}", e));
    info!("Built tokio runtime");
    info!("Entering tokio runtime...");
    runtime.block_on(async {
        run_rvoc_backend(&configuration)
            .await
            .unwrap_or_else(|e| error!("Application error: {:#?}", e));
    });

    info!(
        "Tokio runtime returned, shutting down with timeout {}s...",
        configuration.shutdown_timeout.as_secs_f32(),
    );
    runtime.shutdown_timeout(configuration.shutdown_timeout);
    info!("Tokio runtime shut down successfully");

    info!("Terminated");
    Ok(())
}

async fn run_rvoc_backend(configuration: &Configuration) -> RVocResult<()> {
    // check for missing db migrations


    let db_connection_pool = create_async_connection_pool(configuration).await?;

    todo!()
}

async fn create_async_connection_pool(configuration: &Configuration) -> RVocResult<Pool<AsyncPgConnection>> {
    //let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(std::str::from_utf8(configuration.postgres_url.unsecure()).expect("postgres_url should be utf8"));
    //let pool = Pool::builder(manager).build()?;

    // create a new connection pool with the default config
    let connection_manager = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(std::str::from_utf8(configuration.postgres_url.unsecure()).expect("postgres_url should be utf8"));
    let pool = Pool::builder(connection_manager).build()?;

    Ok(pool)
}
