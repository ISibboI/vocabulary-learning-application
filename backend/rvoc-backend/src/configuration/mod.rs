use std::{env::VarError, error::Error, path::PathBuf, str::FromStr, time::Duration};

use crate::error::{RVocError, RVocResult};
use secstr::SecStr;

/// The configuration of the application.
#[derive(Debug)]
pub struct Configuration {
    /// The url to access postgres.
    pub postgres_url: SecStr,

    /// The url to send opentelemetry to.
    pub opentelemetry_url: Option<String>,

    /// The amount of time to wait for processes to shutdown gracefully.
    pub shutdown_timeout: Duration,

    /// The base directory where wiktionary dumps are stored in.
    pub wiktionary_temporary_data_directory: PathBuf,

    /// The batch size to use when inserting words from wiktionary.
    pub wiktionary_dump_insertion_batch_size: usize,

    /// The maximum number of retries when the transaction that inserts words from wiktionary fails.
    pub wiktionary_dump_insertion_maximum_retry_count: u64,
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
            shutdown_timeout: Duration::from_secs(read_env_var_with_default_as_type(
                "RVOC_SHUTDOWN_TIMEOUT",
                30u64,
            )?),
            wiktionary_temporary_data_directory: read_env_var_with_default_as_type(
                "WIKTIONARY_TEMPORARY_DATA_DIRECTORY",
                "wiktionary_data",
            )?,
            wiktionary_dump_insertion_batch_size: read_env_var_with_default_as_type(
                "WIKTIONARY_DUMP_INSERTION_BATCH_SIZE",
                1000usize,
            )?,
            wiktionary_dump_insertion_maximum_retry_count: read_env_var_with_default_as_type(
                "WIKTIONARY_DUMP_INSERTION_MAXIMUM_RETRY_COUNT",
                10u64,
            )?,
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
    <T as FromStr>::Err: 'static + Error,
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
    <T as FromStr>::Err: 'static + Error,
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
