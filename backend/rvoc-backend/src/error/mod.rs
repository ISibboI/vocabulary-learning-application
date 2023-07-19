use std::{error::Error, ffi::OsString, fmt::Display};

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

    DatabaseConnectionPoolCreation {
        cause: diesel_async::pooled_connection::deadpool::BuildError,
    }
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
            RVocError::DatabaseConnectionPoolCreation { cause } => write!(f, "error creating the database connection pool: {cause}"),
        }
    }
}

impl Error for RVocError {
    fn cause(&self) -> Option<&dyn Error> {
        match self {
            RVocError::MissingEnvironmentVariable { .. } => None,
            RVocError::MalformedEnvironmentVariable { cause, .. } => Some(cause.as_ref()),
            RVocError::DatabaseConnectionPoolCreation { cause } => Some(cause),
        }
    }
}

impl From<diesel_async::pooled_connection::deadpool::BuildError> for RVocError {
    fn from(value: diesel_async::pooled_connection::deadpool::BuildError) -> Self {
        Self::DatabaseConnectionPoolCreation { cause: value }
    }
}