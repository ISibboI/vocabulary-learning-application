use std::{env::VarError, error::Error, net::SocketAddr, path::PathBuf, str::FromStr};

use crate::error::{RVocError, RVocResult};
use chrono::Duration;
use secstr::SecStr;

/// The configuration of the application.
#[derive(Debug, Clone)]
pub struct Configuration {
    /// The url to access postgres.
    pub postgres_url: SecStr,

    /// The url to send opentelemetry to.
    pub opentelemetry_url: Option<String>,

    /// The amount of time to wait for processes to shutdown gracefully.
    pub shutdown_timeout: Duration,

    /// The interval at which the job queue will be polled.
    pub job_queue_poll_interval: Duration,

    /// The maximum number of retries for a failed transaction.
    pub maximum_transaction_retry_count: u64,

    /// The address to listen for API requests.
    pub api_listen_address: SocketAddr,

    /// The base directory where wiktionary dumps are stored in.
    pub wiktionary_temporary_data_directory: PathBuf,

    /// The batch size to use when inserting words from wiktionary.
    pub wiktionary_dump_insertion_batch_size: usize,

    /// The interval at which wiktionary is polled for new dumps, and the dumps are integrated if there is a new one.
    pub wiktionary_update_interval: Duration,
}

impl Configuration {
    /// Read the configuration values from environment variables.
    pub fn from_environment() -> RVocResult<Self> {
        Ok(Self {
            postgres_url: read_env_var_with_default_as_type(
                "POSTGRES_RVOC_URL",
                "postgres://rvoc@localhost/rvoc",
            )?,
            opentelemetry_url: read_optional_env_var("OPENTELEMETRY_URL")?,
            shutdown_timeout: Duration::seconds(read_env_var_with_default_as_type(
                "RVOC_SHUTDOWN_TIMEOUT",
                30i64,
            )?),
            job_queue_poll_interval: Duration::seconds(read_env_var_with_default_as_type(
                "JOB_QUEUE_POLL_INTERVAL_SECONDS",
                60i64,
            )?),
            maximum_transaction_retry_count: read_env_var_with_default_as_type(
                "MAXIMUM_TRANSACTION_RETRY_COUNT",
                10u64,
            )?,
            api_listen_address: read_env_var_with_default_as_type(
                "API_LISTEN_ADDRESS",
                SocketAddr::from(([0, 0, 0, 0], 8093)),
            )?,
            wiktionary_temporary_data_directory: read_env_var_with_default_as_type(
                "WIKTIONARY_TEMPORARY_DATA_DIRECTORY",
                "wiktionary_data",
            )?,
            wiktionary_dump_insertion_batch_size: read_env_var_with_default_as_type(
                "WIKTIONARY_DUMP_INSERTION_BATCH_SIZE",
                1000usize,
            )?,
            wiktionary_update_interval: Duration::hours(read_env_var_with_default_as_type::<i64>(
                "WIKTIONARY_POLL_INTERVAL_HOURS",
                24,
            )?),
        })
    }
}

#[allow(dead_code)]
fn read_env_var(key: &str) -> RVocResult<String> {
    std::env::var(key).map_err(|error| match error {
        VarError::NotPresent => RVocError::MissingEnvironmentVariable {
            key: key.to_string(),
        },
        VarError::NotUnicode(value) => RVocError::MalformedEnvironmentVariable {
            key: key.to_string(),
            value: value.clone(),
            source: Box::new(VarError::NotUnicode(value)),
        },
    })
}

fn read_optional_env_var(key: &str) -> RVocResult<Option<String>> {
    match std::env::var(key) {
        Ok(value) => Ok(Some(value)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(value)) => Err(RVocError::MalformedEnvironmentVariable {
            key: key.to_string(),
            value: value.clone(),
            source: Box::new(VarError::NotUnicode(value)),
        }),
    }
}

#[allow(dead_code)]
fn read_env_var_as_type<T: FromStr>(key: &str) -> RVocResult<T>
where
    <T as FromStr>::Err: 'static + Error + Send + Sync,
{
    match std::env::var(key) {
        Ok(value) => value
            .parse()
            .map_err(|error| RVocError::MalformedEnvironmentVariable {
                key: key.to_string(),
                value: value.into(),
                source: Box::new(error),
            }),
        Err(VarError::NotPresent) => Err(RVocError::MissingEnvironmentVariable {
            key: key.to_string(),
        }),
        Err(VarError::NotUnicode(value)) => Err(RVocError::MalformedEnvironmentVariable {
            key: key.to_string(),
            value: value.clone(),
            source: Box::new(VarError::NotUnicode(value)),
        }),
    }
}

#[allow(dead_code)]
fn read_env_var_with_default(key: &str, default: impl Into<String>) -> RVocResult<String> {
    match std::env::var(key) {
        Ok(value) => Ok(value),
        Err(VarError::NotPresent) => Ok(default.into()),
        Err(VarError::NotUnicode(value)) => Err(RVocError::MalformedEnvironmentVariable {
            key: key.to_string(),
            value: value.clone(),
            source: Box::new(VarError::NotUnicode(value)),
        }),
    }
}

fn read_env_var_with_default_as_type<T: FromStr>(key: &str, default: impl Into<T>) -> RVocResult<T>
where
    <T as FromStr>::Err: 'static + Error + Send + Sync,
{
    match std::env::var(key) {
        Ok(value) => value
            .parse()
            .map_err(|error| RVocError::MalformedEnvironmentVariable {
                key: key.to_string(),
                value: value.into(),
                source: Box::new(error),
            }),
        Err(VarError::NotPresent) => Ok(default.into()),
        Err(VarError::NotUnicode(value)) => Err(RVocError::MalformedEnvironmentVariable {
            key: key.to_string(),
            value: value.clone(),
            source: Box::new(VarError::NotUnicode(value)),
        }),
    }
}
