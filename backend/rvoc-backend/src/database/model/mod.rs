use crate::RVocResult;
use log::info;
use serde::{Deserialize, Serialize};
use wither::bson::oid::ObjectId;
use wither::mongodb::bson::doc;
use wither::mongodb::Database;
use wither::Model;

#[derive(Debug, Model, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{"name": 1}"#, options = r#"doc!{"unique": true}"#))]
pub struct Language {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    name: String,
}

#[derive(Debug, Model, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{"language_name": 1}"#))]
pub struct Word {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    language_name: String,
    word: String,
}

pub async fn sync_model(database: &Database) -> RVocResult<()> {
    info!("Syncing database model...");
    for result in [Language::sync(database), Word::sync(database)] {
        result.await?;
    }
    info!("Synced database model successfully");
    Ok(())
}
