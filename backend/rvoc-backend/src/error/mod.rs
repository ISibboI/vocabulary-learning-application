use crate::database::model::users::SessionId;
use std::net::AddrParseError;
use std::path::PathBuf;
use wither::WitherError;

pub type RVocResult<T> = Result<T, RVocError>;

#[derive(Debug)]
pub enum RVocError {
    // Wrapped errors
    WitherError(WitherError),
    MongoDBError(wither::mongodb::error::Error),
    AddrParseError(AddrParseError),
    IoError(std::io::Error),
    TomlDeserializeError(toml::de::Error),
    PasswordHashError(password_hash::Error),
    BsonSerializeError(wither::bson::ser::Error),

    // Custom errors
    /// A config file was given, but the file extension is not supported.
    UnsupportedConfigFileExtension(PathBuf),

    /// A password passed to the application was too long.
    PasswordTooLong {
        /// The number of bytes in the given password.
        password_bytes: usize,
        /// The maximum allowed number of bytes.
        max_bytes: usize,
    },

    /// Could not create the client type for the database.
    CouldNotSetUpDatabaseClient(wither::mongodb::error::Error),

    /// Could not sync the database model specified by the application with the database.
    CouldNotSyncDatabaseModel(WitherError),

    /// The given session id is not part of any user.
    SessionIdNotFound(SessionId),

    /// The given session id has a length different to the configured one.
    WrongSessionIdLength {
        given_session_id_length: usize,
        configured_session_id_length: usize,
    },

    /// The given string is not a valid session id (i.e. contains invalid characters).
    InvalidSessionId(String),

    /// The user is not authenticated.
    NotAuthenticated,

    /// Could not find the given login name.
    LoginNameNotFound(String),

    /// Cannot update the current session expiry.
    CannotUpdateSessionExpiry,
}

impl From<WitherError> for RVocError {
    fn from(error: WitherError) -> Self {
        Self::WitherError(error)
    }
}

impl From<wither::mongodb::error::Error> for RVocError {
    fn from(error: wither::mongodb::error::Error) -> Self {
        Self::MongoDBError(error)
    }
}

impl From<AddrParseError> for RVocError {
    fn from(error: AddrParseError) -> Self {
        Self::AddrParseError(error)
    }
}

impl From<std::io::Error> for RVocError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error)
    }
}

impl From<toml::de::Error> for RVocError {
    fn from(error: toml::de::Error) -> Self {
        Self::TomlDeserializeError(error)
    }
}

impl From<password_hash::Error> for RVocError {
    fn from(error: password_hash::Error) -> Self {
        Self::PasswordHashError(error)
    }
}

impl From<wither::bson::ser::Error> for RVocError {
    fn from(error: wither::bson::ser::Error) -> Self {
        Self::BsonSerializeError(error)
    }
}
