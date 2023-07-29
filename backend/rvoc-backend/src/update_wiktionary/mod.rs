use tokio::fs;

use crate::error::RVocResult;
use crate::{configuration::Configuration, error::RVocError};
use tracing::{debug, error, instrument};
use wiktionary_dump_parser::{language_code::LanguageCode, urls::DumpBaseUrl};

#[instrument(err, skip(configuration))]
pub async fn run_update_wiktionary(configuration: &Configuration) -> RVocResult<()> {
    debug!("Updating wiktionary data with configuration: {configuration:#?}");

    let target_directory = &configuration.wiktionary_temporary_data_directory;
    if !target_directory.exists() {
        if !target_directory.is_dir() {
            return Err(RVocError::DataDirectoryIsFile {
                path: target_directory.to_owned(),
            });
        }

        fs::create_dir_all(&target_directory)
            .await
            .map_err(|error| RVocError::CreateDirectory {
                path: target_directory.clone(),
                cause: Box::new(error),
            })?;
    }

    let new_dump_file = wiktionary_dump_parser::download_language(
        &DumpBaseUrl::Default,
        &LanguageCode::English,
        target_directory,
        10,
    )
    .await
    .map_err(|error| RVocError::DownloadLanguage {
        cause: Box::new(error),
    })?;

    // TODO remove old dump files
    if let Some(dump_file_base_directory) = new_dump_file.ancestors().skip(2).next() {
        debug!("Removing old dump files");
        let new_directory_name = new_dump_file.parent().unwrap().file_name().unwrap();

        let mut base_directory_iterator =
            fs::read_dir(dump_file_base_directory)
                .await
                .map_err(|error| RVocError::DeleteOldWiktionaryDumps {
                    cause: Box::new(error),
                })?;
        let mut deletables = Vec::new();

        while let Some(entry) = base_directory_iterator
            .next_entry()
            .await
            .map_err(|error| RVocError::DeleteOldWiktionaryDumps {
                cause: Box::new(error),
            })?
        {
            if entry.file_name() != new_directory_name {
                deletables.push(entry.file_name());
            }
        }

        for directory in deletables {
            let mut delete_path = dump_file_base_directory.to_path_buf();
            delete_path.push(directory);
            if let Err(error) = fs::remove_dir_all(&delete_path).await {
                // Aborting here seems unnecessary, as deleting other directories may still succeed.
                // So instead we just log the error and proceed.
                error!("Could not delete old dump file directory {delete_path:?}: {error}");
            }
        }
    } else {
        // If this is reached then the directory convention of wiktionary-dump-parser has changed,
        // so it would be a programming error. But it does not make sense to panic here, because
        // not being able to delete old dump files is not that bad.
        error!("New dump file has no base directory: {new_dump_file:?}");
    };

    Ok(())
}
