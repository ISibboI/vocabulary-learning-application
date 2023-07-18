use std::net::AddrParseError;
use std::path::PathBuf;

pub type RVocResult<T> = Result<T, RVocError>;

#[derive(Debug)]
pub enum RVocError {
    // Wrapped errors
    AddrParseError(AddrParseError),
    IoError(std::io::Error),
    TomlDeserializeError(toml::de::Error),
    PasswordHashError(password_hash::Error),

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

    /// Cannot delete a certain session id of a user.
    CannotDeleteSession,

    /// Cannot delete all session ids of a user but a given one.
    CannotDeleteOtherSessions,

    /// Cannot delete all sessions of a user.
    CannotDeleteAllSessions,

    /// Did not find a free session id after the given amount of attempts.
    NoFreeSessionId {
        attempts: usize,
    },
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