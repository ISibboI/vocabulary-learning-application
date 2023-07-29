use std::{error::Error, ffi::OsString, fmt::Display, path::PathBuf};

pub type RVocResult<T> = Result<T, RVocError>;

#[derive(Debug)]
pub enum RVocError {
    MissingEnvironmentVariable {
        key: String,
    },

    MalformedEnvironmentVariable {
        key: String,
        value: OsString,
        cause: Box<dyn Error>,
    },

    SetupTracing {
        cause: Box<dyn Error>,
    },

    DatabaseConnectionPoolCreation {
        cause: diesel_async::pooled_connection::deadpool::BuildError,
    },

    DatabaseMigration {
        cause: Box<dyn Error>,
    },

    DataDirectoryIsFile {
        path: PathBuf,
    },

    CreateDirectory {
        path: PathBuf,
        cause: Box<dyn Error>,
    },

    DownloadLanguage {
        cause: Box<dyn Error>,
    },

    DeleteOldWiktionaryDumps {
        cause: Box<dyn Error>,
    },
}

impl Display for RVocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RVocError::MissingEnvironmentVariable { key: name } => {
                write!(f, "missing environment variable '{name}'")
            }
            RVocError::MalformedEnvironmentVariable { key, value, cause } => write!(
                f,
                "environment variable '{key}' has malformed value {value:?} caused by: {cause}"
            ),
            RVocError::SetupTracing { cause } => write!(f, "setting up tracing failed: {cause}"),
            RVocError::DatabaseConnectionPoolCreation { cause } => {
                write!(f, "error creating the database connection pool: {cause}")
            }
            RVocError::DatabaseMigration { cause } => {
                write!(f, "error executing the database migrations: {cause}")
            }
            RVocError::DataDirectoryIsFile { path } => write!(
                f,
                "data directory should be a directory, but is a file: {path:?}"
            ),
            RVocError::CreateDirectory { path, cause } => {
                write!(f, "error creating directory {path:?}: {cause}")
            }
            RVocError::DownloadLanguage { cause } => {
                write!(f, "error downloading language: {cause}")
            }
            RVocError::DeleteOldWiktionaryDumps { cause } => write!(f, "error deleting old wiktionary dumps: {cause}"),
        }
    }
}

impl Error for RVocError {
    fn cause(&self) -> Option<&dyn Error> {
        match self {
            RVocError::MissingEnvironmentVariable { .. } => None,
            RVocError::MalformedEnvironmentVariable { cause, .. } => Some(cause.as_ref()),
            RVocError::SetupTracing { cause } => Some(cause.as_ref()),
            RVocError::DatabaseConnectionPoolCreation { cause } => Some(cause),
            RVocError::DatabaseMigration { cause } => Some(cause.as_ref()),
            RVocError::DataDirectoryIsFile { .. } => None,
            RVocError::CreateDirectory { cause, .. } => Some(cause.as_ref()),
            RVocError::DownloadLanguage { cause } => Some(cause.as_ref()),
            RVocError::DeleteOldWiktionaryDumps { cause } => Some(cause.as_ref()),
        }
    }
}

impl From<diesel_async::pooled_connection::deadpool::BuildError> for RVocError {
    fn from(value: diesel_async::pooled_connection::deadpool::BuildError) -> Self {
        Self::DatabaseConnectionPoolCreation { cause: value }
    }
}
