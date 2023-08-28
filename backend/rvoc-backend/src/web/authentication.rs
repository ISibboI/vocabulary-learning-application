use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use typed_session_axum::ReadableSession;

use super::{session::RVocSessionData, user::model::Username};

pub async fn ensure_logged_in<B>(mut request: Request<B>, next: Next<B>) -> Response {
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

impl AsRef<str> for LoggedInUser {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
