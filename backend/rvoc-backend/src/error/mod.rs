use std::{error::Error, ffi::OsString, path::PathBuf};

use thiserror::Error;

pub type RVocResult<T> = Result<T, RVocError>;
pub type BoxDynError = Box<dyn Error + Send + Sync>;

#[derive(Debug, Error)]
pub enum RVocError {
    // Configuration errors
    #[error("missing environment variable '{key}'")]
    MissingEnvironmentVariable { key: String },

    #[error("environment variable '{key}' has malformed value {value:?} caused by: {source}")]
    MalformedEnvironmentVariable {
        key: String,
        value: OsString,
        source: BoxDynError,
    },

    #[error("setting up tracing failed: {source}")]
    SetupTracing { source: BoxDynError },

    #[error("error creating the database connection pool: {source}")]
    DatabaseConnectionPoolCreation {
        #[from]
        source: diesel_async::pooled_connection::deadpool::BuildError,
    },

    #[error("could not connect to the database: {source}")]
    DatabaseConnection { source: BoxDynError },

    #[error("permanent database transaction error: {source}")]
    PermanentDatabaseTransactionError { source: BoxDynError },

    #[error(
        "the maximum number of retries for retrying a database transaction was reached (limit: {limit})"
    )]
    DatabaseTransactionRetryLimitReached { limit: u64 },

    #[error("error executing the database migrations: {source}")]
    DatabaseMigration { source: BoxDynError },

    #[error("error while serving API request: {source}")]
    ApiServerError { source: BoxDynError },

    #[error(
        "the password pepper string's length ({actual}) is out of range: [{minimum}, {maximum}]"
    )]
    PasswordPepperLength {
        actual: usize,
        minimum: usize,
        maximum: usize,
    },

    #[error("the minimum password length is too low: {actual} < {minimum}")]
    MinimumPasswordLength { actual: usize, minimum: usize },

    #[error("the parameters to the argon password function are wrong: {error}")]
    PasswordArgon2IdParameters { error: argon2::Error },

    #[error("password hashing went wrong: {source}")]
    PasswordArgon2IdHash { source: BoxDynError },

    #[error("error while inserting session to database: {source}")]
    InsertSession { source: BoxDynError },

    #[error("error while reading the session from the database: {source}")]
    ReadSession { source: BoxDynError },

    #[error("data directory should be a directory, but is a file: {path:?}")]
    DataDirectoryIsFile { path: PathBuf },

    #[error("error creating directory {path:?}: {source}")]
    CreateDirectory { path: PathBuf, source: BoxDynError },

    #[error("error downloading wiktionary dump: {source}")]
    DownloadWiktionaryDump { source: BoxDynError },

    #[error("error deleting old wiktionary dumps: {source}")]
    DeleteOldWiktionaryDumps { source: BoxDynError },

    #[error("error parsing wiktionary dump file: {source}")]
    ParseWiktionaryDump { source: BoxDynError },

    #[error("there are pending database migrations")]
    PendingDatabaseMigrations,

    #[error("could not access the job queue: {source}")]
    AccessJobQueue { source: BoxDynError },

    #[error("could not join tokio task: {source}")]
    TokioTaskJoin { source: BoxDynError },
}

trait RequireSendAndSync: Send + Sync {}
impl RequireSendAndSync for RVocError {}
