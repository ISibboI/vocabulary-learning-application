use crate::error::{RVocError, RVocResult};
use clap::Parser;
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Borrow;
use std::fs::read_to_string;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Clone, Serialize, Deserialize)]
#[clap(version = "0.1.0", author = "Sebastian Schmidt <isibboi@gmail.com>")]
#[serde(rename_all = "kebab-case")]
pub struct ConfigurationInner {
    #[clap(long, help = "Path to the config file")]
    pub config_file: Option<PathBuf>,

    #[clap(long, default_value = "1", conflicts_with = "config-file")]
    pub tokio_worker_threads: usize,

    #[clap(
        long,
        default_value = "5",
        help = "The shutdown timeout for the tokio runtime in seconds",
        conflicts_with = "config-file"
    )]
    pub tokio_shutdown_timeout_seconds: u64,

    #[clap(
        long,
        default_value = "mongodb://root:test@localhost:27017",
        conflicts_with = "config-file"
    )]
    pub mongodb_uri: String,

    #[clap(long, default_value = "localhost", conflicts_with = "config-file")]
    pub mongodb_host: String,

    #[clap(long, default_value = "27017", conflicts_with = "config-file")]
    pub mongodb_port: u16,

    #[clap(long, default_value = "root", conflicts_with = "config-file")]
    pub mongodb_user: String,

    #[clap(long, default_value = "test", conflicts_with = "config-file")]
    pub mongodb_password: String,

    #[clap(long, default_value = "rvoc", conflicts_with = "config-file")]
    pub mongodb_database: String,

    #[clap(long, default_value = "10", conflicts_with = "config-file")]
    pub mongodb_connect_timeout: u64,

    #[clap(long, default_value = "0.0.0.0", conflicts_with = "config-file")]
    pub api_listen_address: String,

    #[clap(long, default_value = "2374", conflicts_with = "config-file")]
    pub api_listen_port: u16,

    #[clap(long, default_value = "1024", conflicts_with = "config-file")]
    pub max_password_bytes: usize,

    #[clap(long, default_value = "32", conflicts_with = "config-file")]
    pub session_id_length: usize,

    #[clap(long, default_value = "604800", conflicts_with = "config-file")]
    pub session_cookie_max_age_seconds: i64,
}

#[derive(Clone)]
pub struct Configuration {
    inner: Arc<ConfigurationInner>,
}

impl Configuration {
    pub fn parse() -> Self {
        Self {
            inner: Arc::new(ConfigurationInner::parse()),
        }
    }

    #[cfg(test)]
    pub fn parse_from<I, T>(iterator: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        Self {
            inner: Arc::new(ConfigurationInner::parse_from(iterator)),
        }
    }
}

impl AsRef<ConfigurationInner> for Configuration {
    fn as_ref(&self) -> &ConfigurationInner {
        self.inner.as_ref()
    }
}

impl Borrow<ConfigurationInner> for Configuration {
    fn borrow(&self) -> &ConfigurationInner {
        self.inner.borrow()
    }
}

impl Deref for Configuration {
    type Target = ConfigurationInner;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'de> Deserialize<'de> for Configuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ConfigurationInner::deserialize(deserializer).map(|configuration_inner| Configuration {
            inner: Arc::new(configuration_inner),
        })
    }
}

/// Parse the configuration.
/// This method either returns the configuration given on the command line,
/// or, if the `--config-file` argument is set, it reads the given file and returns that config instead.
pub fn parse_configuration() -> RVocResult<Configuration> {
    let configuration = Configuration::parse();

    if let Some(config_file) = &configuration.config_file {
        if config_file.ends_with(".toml") {
            let config_file_content = read_to_string(config_file)?;
            Ok(toml::from_str(&config_file_content)?)
        } else {
            Err(RVocError::UnsupportedConfigFileExtension(
                config_file.clone(),
            ))
        }
    } else {
        Ok(configuration)
    }
}
