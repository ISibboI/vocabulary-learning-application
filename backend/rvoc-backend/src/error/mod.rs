use std::{error::Error, ffi::OsString, path::PathBuf};

use thiserror::Error;

pub type RVocResult<T> = Result<T, RVocError>;

#[derive(Debug, Error)]
pub enum RVocError {
    // Configuration errors
    #[error("missing environment variable '{key}'")]
    MissingEnvironmentVariable { key: String },

    #[error("environment variable '{key}' has malformed value {value:?} caused by: {source}")]
    MalformedEnvironmentVariable {
        key: String,
        value: OsString,
        source: Box<dyn Error>,
    },

    #[error("setting up tracing failed: {source}")]
    SetupTracing { source: Box<dyn Error> },

    #[error("error creating the database connection pool: {source}")]
    DatabaseConnectionPoolCreation {
        #[from]
        source: diesel_async::pooled_connection::deadpool::BuildError,
    },

    #[error("could not connect to the database: {source}")]
    DatabaseConnection { source: Box<dyn Error> },

    #[error("error executing the database migrations: {source}")]
    DatabaseMigration { source: Box<dyn Error> },

    #[error("data directory should be a directory, but is a file: {path:?}")]
    DataDirectoryIsFile { path: PathBuf },

    #[error("error creating directory {path:?}: {source}")]
    CreateDirectory {
        path: PathBuf,
        source: Box<dyn Error>,
    },

    #[error("error downloading wiktionary dump: {source}")]
    DownloadWiktionaryDump { source: Box<dyn Error> },

    #[error("error deleting old wiktionary dumps: {source}")]
    DeleteOldWiktionaryDumps { source: Box<dyn Error> },

    #[error("error parsing wiktionary dump file: {source}")]
    ParseWiktionaryDump { source: Box<dyn Error> },

    #[error("there are pending database migrations")]
    PendingDatabaseMigrations,
}
