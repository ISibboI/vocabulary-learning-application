use std::{env::VarError, error::Error, str::FromStr, time::Duration};

use crate::error::{RVocError, RVocResult};
use secstr::SecStr;

/// The configuration of the application.
#[derive(Debug)]
pub struct Configuration {
    /// The url to access postgres.
    pub postgres_url: SecStr,

    /// The amount of time to wait for processes to shutdown gracefully.
    pub shutdown_timeout: Duration,
}

impl Configuration {
    /// Read the configuration values from environment variables.
    pub fn from_environment() -> RVocResult<Self> {
        Ok(Self {
            postgres_url: read_env_var_with_default_as_type(
                "POSTGRES_RVOC_URL",
                "postgres://postgres@localhost/rvoc_dev".into(),
            )?,
            shutdown_timeout: Duration::from_secs(read_env_var_with_default_as_type(
                "RVOC_SHUTDOWN_TIMEOUT",
                30,
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
            cause: Box::new(VarError::NotUnicode(value)),
        },
    })
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
                cause: Box::new(error),
            }),
        Err(VarError::NotPresent) => Err(RVocError::MissingEnvironmentVariable {
            key: key.to_string(),
        }),
        Err(VarError::NotUnicode(value)) => Err(RVocError::MalformedEnvironmentVariable {
            key: key.to_string(),
            value: value.clone(),
            cause: Box::new(VarError::NotUnicode(value)),
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
            cause: Box::new(VarError::NotUnicode(value)),
        }),
    }
}

fn read_env_var_with_default_as_type<T: FromStr>(key: &str, default: T) -> RVocResult<T>
where
    <T as FromStr>::Err: 'static + Error,
{
    match std::env::var(key) {
        Ok(value) => value
            .parse()
            .map_err(|error| RVocError::MalformedEnvironmentVariable {
                key: key.to_string(),
                value: value.into(),
                cause: Box::new(error),
            }),
        Err(VarError::NotPresent) => Ok(default),
        Err(VarError::NotUnicode(value)) => Err(RVocError::MalformedEnvironmentVariable {
            key: key.to_string(),
            value: value.clone(),
            cause: Box::new(VarError::NotUnicode(value)),
        }),
    }
}
