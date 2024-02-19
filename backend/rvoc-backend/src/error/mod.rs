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

    #[error("the configured shutdown timeout is negative")]
    NegativeShutdownTimeout,

    #[error("the configured job queue poll interval is negative")]
    NegativeJobQueuePollInterval,

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

    #[error("{0}")]
    UserError(
        #[from]
        #[source]
        UserError,
    ),

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

    #[error("the parameters to the argon password function are wrong: {source}")]
    PasswordArgon2IdParameters { source: BoxDynError },

    #[error("password hashing went wrong: {source}")]
    PasswordArgon2IdHash { source: BoxDynError },

    #[error("password verification went wrong: {source}")]
    PasswordArgon2IdVerify { source: BoxDynError },

    #[error("password rehashing went wrong: {source}")]
    PasswordArgon2IdRehash { source: BoxDynError },

    #[error("error creating user: {source}")]
    CreateUser { source: BoxDynError },

    #[error("error deleting user: {source}")]
    DeleteUser { source: BoxDynError },

    #[error("error expiring all passwords: {source}")]
    ExpireAllPasswords { source: BoxDynError },

    #[error("error expiring all sessions: {source}")]
    ExpireAllSessions { source: BoxDynError },

    #[error("error reading password from stdin: {source}")]
    ReadPasswordFromStdin { source: BoxDynError },

    #[error("error deleting all user sessions: {source}")]
    DeleteAllUserSessions { source: BoxDynError },

    #[error("error logging in: {source}")]
    Login { source: BoxDynError },

    #[error("error while inserting a session to the database: {source}")]
    InsertSession { source: BoxDynError },

    #[error("error while reading a session from the database: {source}")]
    ReadSession { source: BoxDynError },

    #[error("error while updating a session in the database: {source}")]
    UpdateSession { source: BoxDynError },

    #[error("error while removing a session from the database: {source}")]
    DeleteSession { source: BoxDynError },

    #[error("error while removing all sessiona from the database: {source}")]
    DeleteAllSessions { source: BoxDynError },

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

#[derive(Debug, Error)]
pub enum UserError {
    #[error("password length ({actual}) outside of allowed range [{minimum}, {maximum}]")]
    PasswordLength {
        actual: usize,
        minimum: usize,
        maximum: usize,
    },

    #[error("username length ({actual}) outside of allowed range [{minimum}, {maximum}]")]
    UsernameLength {
        actual: usize,
        minimum: usize,
        maximum: usize,
    },

    #[error("the username already exists: {username}")]
    UsernameExists { username: String },

    #[error("the username does not exist: {username}")]
    UsernameDoesNotExist { username: String },

    #[error("the username or password did not match")]
    InvalidUsernamePassword,
}

#[allow(dead_code)]
trait RequireSendAndSync: Send + Sync {}
impl RequireSendAndSync for RVocError {}
