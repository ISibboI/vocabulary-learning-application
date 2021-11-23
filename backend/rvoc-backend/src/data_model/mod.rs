use log::info;
use wither::bson::oid::ObjectId;
use wither::mongodb::Database;
use wither::Model;
use serde::{Serialize, Deserialize};
use crate::RVocResult;

#[derive(Debug, Model, Serialize, Deserialize)]
//#[model(index(keys=r#"doc!{"email": 1}"#, options=r#"doc!{"unique": true}"#))]
pub struct Language {
    #[serde(rename="_id", skip_serializing_if="Option::is_none")]
    id: Option<ObjectId>,
    name: String,
}

pub async fn sync_model(database: &Database) -> RVocResult<()> {
    info!("Syncing database model...");
    for result in [Language::sync(database)] {
        result.await?;
    }
    info!("Synced database model successfully");
    Ok(())
}