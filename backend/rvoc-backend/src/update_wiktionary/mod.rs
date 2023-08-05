use std::path::PathBuf;

use diesel::{ExpressionMethods, NullableExpressionMethods, RunQueryDsl};
use tokio::fs;
use tracing::{debug, error, info, instrument, warn};
use wiktionary_dump_parser::parser::parse_dump_file;
use wiktionary_dump_parser::{language_code::LanguageCode, urls::DumpBaseUrl};

use crate::database::create_sync_database_connection;
use crate::error::RVocResult;
use crate::{configuration::Configuration, error::RVocError};

#[instrument(err, skip(configuration))]
pub async fn run_update_wiktionary(configuration: &Configuration) -> RVocResult<()> {
    info!("Updating wiktionary data");
    debug!("Configuration: {configuration:#?}");

    let new_dump_file = update_wiktionary_dump_files(configuration).await?;
    // expect the extension to be ".tar.bz2", and replace it with ".log"
    let error_log = new_dump_file.with_extension("").with_extension("log");
    let mut database_connection = create_sync_database_connection(configuration)?;

    debug!("Parsing wiktionary dump file {new_dump_file:?}");
    let mut word_buffer = Vec::new();
    parse_dump_file(
        new_dump_file,
        Option::<PathBuf>::None,
        |word| {
            word_buffer.push(word);

            let mut tries = 0;
            while word_buffer.len() >= configuration.wiktionary_dump_insertion_batch_size {
                debug!("Inserting {} words into database", word_buffer.len());

                tries += 1;
                if tries > configuration.wiktionary_dump_insertion_maximum_retry_count {
                    return Err(Box::new(
                        RVocError::WiktionaryDumpInsertionTransactionLimitReached,
                    ));
                }

                let transaction_result: Result<(), diesel::result::Error> = database_connection
                    .build_transaction()
                    .serializable()
                    .run(|database_connection| {
                        {
                            use crate::schema::*;

                            diesel::insert_into(languages::table)
                                .values(
                                    &word_buffer
                                        .iter()
                                        .map(|word| {
                                            languages::english_name.eq(&word.language_english_name)
                                        })
                                        .collect::<Vec<_>>(),
                                )
                                .on_conflict_do_nothing()
                                .execute(database_connection)?;

                            diesel::insert_into(word_types::table)
                                .values(
                                    &word_buffer
                                        .iter()
                                        .map(|word| word_types::english_name.eq(&word.word_type))
                                        .collect::<Vec<_>>(),
                                )
                                .on_conflict_do_nothing()
                                .execute(database_connection)?;
                        }

                        // query:
                        // INSERT INTO words (word, word_type, language) VALUES (
                        //    "...",
                        //    (SELECT id FROM word_types WHERE english_name = "..."),
                        //    (SELECT id FROM languages WHERE english_name = "...")
                        // );

                        use crate::schema::*;
                        use diesel::QueryDsl;

                        diesel::insert_into(words::table)
                            .values(
                                word_buffer
                                    .iter()
                                    .map(|word| {
                                        (
                                            words::word.eq(&word.word),
                                            words::language.eq(languages::table
                                                .select(languages::id)
                                                .filter(
                                                    languages::english_name
                                                        .eq(&word.language_english_name),
                                                )
                                                .single_value()
                                                .assume_not_null()),
                                            words::word_type.eq(word_types::table
                                                .select(word_types::id)
                                                .filter(
                                                    word_types::english_name.eq(&word.word_type),
                                                )
                                                .single_value()
                                                .assume_not_null()),
                                        )
                                    })
                                    .collect::<Vec<_>>(),
                            )
                            .on_conflict_do_nothing()
                            .execute(database_connection)?;

                        Ok(())
                    });

                match transaction_result {
                    Ok(()) => {
                        word_buffer.clear();
                        break;
                    }
                    Err(error @ diesel::result::Error::RollbackErrorOnCommit { .. }) => {
                        return Err(Box::new(error))
                    }
                    Err(error) => {
                        warn!("Word insertion transaction unsuccessful, retrying: {error}")
                    }
                }
            }

            Ok(())
        },
        error_log,
        false,
    )
    .await
    .map_err(|error| RVocError::ParseWiktionaryDump {
        source: Box::new(error),
    })?;

    info!("Success!");

    Ok(())
}

#[instrument(err, skip(configuration))]
async fn update_wiktionary_dump_files(configuration: &Configuration) -> RVocResult<PathBuf> {
    debug!("Updating wiktionary dump files");
    let target_directory = &configuration.wiktionary_temporary_data_directory;
    if !target_directory.exists() {
        fs::create_dir_all(&target_directory)
            .await
            .map_err(|error| RVocError::CreateDirectory {
                path: target_directory.clone(),
                source: Box::new(error),
            })?;
    } else if !target_directory.is_dir() {
        return Err(RVocError::DataDirectoryIsFile {
            path: target_directory.to_owned(),
        });
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
