use std::path::PathBuf;

use diesel::{Connection, Insertable, RunQueryDsl};
use tokio::fs;
use wiktionary_dump_parser::parser::parse_dump_file;

use crate::database::create_sync_database_connection;
use crate::database::model::{InsertLanguage, InsertWordType};
use crate::error::RVocResult;
use crate::{configuration::Configuration, error::RVocError};
use tracing::{debug, error, instrument};
use wiktionary_dump_parser::{language_code::LanguageCode, urls::DumpBaseUrl};

#[instrument(err, skip(configuration))]
pub async fn run_update_wiktionary(configuration: &Configuration) -> RVocResult<()> {
    debug!("Updating wiktionary data with configuration: {configuration:#?}");

    let new_dump_file = update_wiktionary_dump_files(configuration).await?;
    // expect the extension to be ".tar.bz2", and replace it with ".log"
    let error_log = new_dump_file.with_extension("").with_extension(".log");
    let mut database_connection = create_sync_database_connection(configuration)?;

    debug!("Parsing wiktionary dump file {new_dump_file:?}");
    parse_dump_file(
        new_dump_file,
        Option::<PathBuf>::None,
        |word| {
            database_connection.transaction(|connection| {
                InsertLanguage {
                    english_name: word.language_english_name,
                }
                .insert_into(crate::schema::languages::table)
                .on_conflict_do_nothing()
                .execute(connection)?;

                InsertWordType {
                    english_name: word.word_type,
                }
                .insert_into(crate::schema::word_types::table)
                .on_conflict_do_nothing()
                .execute(connection)?;

                todo!()
            })
        },
        error_log,
        false,
    )
    .await
    .map_err(|error| RVocError::ParseWiktionaryDump {
        source: Box::new(error),
    })?;

    Ok(())
}

#[instrument(err, skip(configuration))]
async fn update_wiktionary_dump_files(configuration: &Configuration) -> RVocResult<PathBuf> {
    debug!("Updating wiktionary dump files");
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
                source: Box::new(error),
            })?;
    }

    let new_dump_file = wiktionary_dump_parser::download_language(
        &DumpBaseUrl::Default,
        &LanguageCode::English,
        target_directory,
        10,
    )
    .await
    .map_err(|error| RVocError::DownloadWiktionaryDump {
        source: Box::new(error),
    })?;

    if let Some(dump_file_base_directory) = new_dump_file.ancestors().nth(2) {
        debug!("Removing old dump files");
        let new_directory_name = new_dump_file.parent().unwrap().file_name().unwrap();

        let mut base_directory_iterator =
            fs::read_dir(dump_file_base_directory)
                .await
                .map_err(|error| RVocError::DeleteOldWiktionaryDumps {
                    source: Box::new(error),
                })?;
        let mut deletables = Vec::new();

        while let Some(entry) = base_directory_iterator
            .next_entry()
            .await
            .map_err(|error| RVocError::DeleteOldWiktionaryDumps {
                source: Box::new(error),
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

    Ok(new_dump_file)
}
