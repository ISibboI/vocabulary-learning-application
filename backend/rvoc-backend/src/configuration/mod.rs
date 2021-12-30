use crate::error::RVocError;
use crate::RVocResult;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::PathBuf;

#[derive(Parser, Clone, Serialize, Deserialize)]
#[clap(version = "0.1.0", author = "Sebastian Schmidt <isibboi@gmail.com>")]
#[serde(rename_all = "kebab-case")]
pub struct Configuration {
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
    pub tokio_shutdown_timeout: u64,

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
}

/// Parse the configuration.
/// This method either returns the configuration given on the command line,
/// or, if the `--config-file` argument is set, it reads the given file and returns that config instead.
pub fn parse_configuration() -> RVocResult<Configuration> {
    let configuration: Configuration = Configuration::parse();

    if let Some(config_file) = configuration.config_file {
        if config_file.ends_with(".toml") {
            let config_file_content = read_to_string(config_file)?;
            Ok(toml::from_str(&config_file_content)?)
        } else {
            Err(RVocError::UnsupportedConfigFileExtension(config_file))
        }
    } else {
        Ok(configuration)
    }
}
