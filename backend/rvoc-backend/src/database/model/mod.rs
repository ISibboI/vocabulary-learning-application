use crate::RVocResult;
use log::info;
use wither::mongodb::Database;
use wither::Model;
use crate::database::model::vocabulary::{Language, Word};

pub mod vocabulary;

pub async fn sync_model(database: &Database) -> RVocResult<()> {
    info!("Syncing database model...");
    for result in [Language::sync(database), Word::sync(database)] {
        result.await?;
    }
    info!("Synced database model successfully");
    Ok(())
}
