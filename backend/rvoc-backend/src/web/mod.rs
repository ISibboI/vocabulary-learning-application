use std::{convert::Infallible, fmt::Display, sync::Arc};

use axum::{
    error_handling::HandleErrorLayer,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Extension, Router,
};
use tower::ServiceBuilder;
use tracing::{debug, error, info, instrument};
use typed_session_axum::{ReadableSession, SessionLayer, SessionLayerError};

use crate::{
    configuration::Configuration,
    database::RVocAsyncDatabaseConnectionPool,
    error::{RVocError, RVocResult, UserError},
    web::{
        session::{RVocSessionData, RVocSessionStoreConnector},
        user::{create_account, delete_account},
    },
};

use self::user::model::Username;

mod session;
mod user;

#[instrument(err, skip(database_connection_pool, configuration))]
pub async fn run_web_api(
    database_connection_pool: RVocAsyncDatabaseConnectionPool,
    configuration: &Configuration,
) -> RVocResult<()> {
    info!("Starting web API");

    async fn handle_session_layer_error<
        SessionStoreConnectorError: Display,
        InnerError: Display,
    >(
        error: SessionLayerError<SessionStoreConnectorError, InnerError>,
    ) -> StatusCode {
        error!("Session layer error: {error}");
        StatusCode::INTERNAL_SERVER_ERROR
    }

    let router = Router::new()
        .route("/accounts/delete", delete(delete_account))
        .layer(middleware::from_fn(ensure_logged_in))
        .route("/", get(hello_world))
        .route("/accounts/create", post(create_account))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(
                    handle_session_layer_error::<RVocError, Infallible>,
                ))
                .layer(SessionLayer::<RVocSessionData, RVocSessionStoreConnector>::new()),
        )
        .layer(Extension(RVocSessionStoreConnector::new(
            database_connection_pool.clone(),
            configuration,
        )))
        .layer(Extension(database_connection_pool))
        .layer(Extension(Arc::new(configuration.clone())));

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

async fn ensure_logged_in<B>(mut request: Request<B>, next: Next<B>) -> Response {
    let session: &ReadableSession<RVocSessionData> = request.extensions().get().unwrap();

    match session.data() {
        RVocSessionData::Anonymous => return StatusCode::UNAUTHORIZED.into_response(),
        RVocSessionData::LoggedIn(username) => {
            let username = username.clone();
            request.extensions_mut().insert(LoggedInUser(username));
        }
    }

    next.run(request).await
}

/// If this extension is found, it means that the request was made by the contained username.
#[derive(Debug, Clone)]
pub struct LoggedInUser(Username);

impl From<LoggedInUser> for String {
    fn from(value: LoggedInUser) -> Self {
        value.0.into()
    }
}

impl IntoResponse for RVocError {
    fn into_response(self) -> axum::response::Response {
        if let RVocError::UserError(user_error) = self {
            user_error.into_response()
        } else {
            error!("Web API error: {self}");

            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

impl IntoResponse for UserError {
    fn into_response(self) -> axum::response::Response {
        (self.status_code(), self.to_string()).into_response()
    }
}

impl UserError {
    fn status_code(&self) -> StatusCode {
        match self {
            UserError::PasswordLength { .. } => StatusCode::BAD_REQUEST,
            UserError::UsernameLength { .. } => StatusCode::BAD_REQUEST,
            UserError::UsernameExists { .. } => StatusCode::CONFLICT,
            UserError::UsernameDoesNotExist { .. } => StatusCode::BAD_REQUEST,
        }
    }
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

type WebConfiguration = Extension<Arc<Configuration>>;
type WebDatabaseConnectionPool = Extension<RVocAsyncDatabaseConnectionPool>;
