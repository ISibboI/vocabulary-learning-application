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

    // Custom errors
    /// A config file was given, but the file extension is not supported.
    UnsupportedConfigFileExtension(PathBuf),

    /// A password passed to the application was too long.
    PasswordTooLong,

    /// Could not create the client type for the database.
    CouldNotSetUpDatabaseClient(wither::mongodb::error::Error),

    /// Could not sync the database model specified by the application with the database.
    CouldNotSyncDatabaseModel(WitherError),
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
