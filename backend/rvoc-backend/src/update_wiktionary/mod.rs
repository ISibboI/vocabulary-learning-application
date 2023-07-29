use crate::error::RVocResult;
use crate::{configuration::Configuration, error::RVocError};
use tokio::fs::create_dir_all;
use tracing::{debug, instrument};
use wiktionary_dump_parser::{language_code::LanguageCode, urls::DumpBaseUrl};

#[instrument(err, skip(configuration))]
pub async fn run_update_wiktionary(configuration: &Configuration) -> RVocResult<()> {
    debug!("Updating wiktionary with configuration: {configuration:#?}");

    let target_directory = &configuration.wiktionary_temporary_data_directory;
    if !target_directory.exists() {
        if !target_directory.is_dir() {
            return Err(RVocError::DataDirectoryIsFile {
                path: target_directory.to_owned(),
            });
        }

        create_dir_all(&target_directory)
            .await
            .map_err(|error| RVocError::CreateDirectory {
                path: target_directory.clone(),
                cause: Box::new(error),
            })?;
    }

    let new_language_file = wiktionary_dump_parser::download_language(
        &DumpBaseUrl::Default,
        &LanguageCode::English,
        target_directory,
        10,
    )
    .await
    .map_err(|error| RVocError::DownloadLanguage {
        cause: Box::new(error),
    })?;

    // TODO remove old language files

    Ok(())
}
