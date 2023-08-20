use axum::{routing::get, Router};
use tracing::{debug, error, info, instrument};

use crate::{
    configuration::Configuration,
    database::RVocAsyncDatabaseConnectionPool,
    error::{RVocError, RVocResult},
};

#[instrument(err, skip(database_connection_pool, configuration))]
pub async fn run_web_api(
    database_connection_pool: RVocAsyncDatabaseConnectionPool,
    configuration: &Configuration,
) -> RVocResult<()> {
    info!("Starting web API");

    let router = Router::new()
        .route("/", get(hello_world))
        .with_state(database_connection_pool);

    debug!(
        "Listening for API requests on {}",
        configuration.api_listen_address
    );
    axum::Server::bind(&configuration.api_listen_address)
        .serve(router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|error| RVocError::ApiServerError {
            source: Box::new(error),
        })?;

    info!("Web API terminated normally");
    Ok(())
}

async fn hello_world() -> &'static str {
    "Hello World!"
}

async fn shutdown_signal() {
    let sigint = async {
        if let Err(error) =
            tokio::signal::ctrl_c()
                .await
                .map_err(|error| RVocError::ApiServerError {
                    source: Box::new(error),
                })
        {
            error!("Error receiving SIGINT: {error}");
        }
    };

    #[cfg(unix)]
    let sigterm = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut handler) => {
                if handler.recv().await.is_none() {
                    error!("Received None from SIGTERM handler. This is unexpected.");
                }
            }
            Err(error) => error!("Error installing SIGTERM handler: {error}"),
        }
    };

    // This future never completes, hence we offer no other means of shutdown on non-unix platforms.
    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    // Shutdown if either signal is received
    tokio::select! {
        _ = sigint => info!("Received SIGINT, shutting down"),
        _ = sigterm => info!("Received SIGTERM, shutting down"),
    }
}
