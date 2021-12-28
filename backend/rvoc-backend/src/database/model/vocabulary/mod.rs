use serde::{Deserialize, Serialize};
use wither::bson::oid::ObjectId;
use wither::mongodb::bson::doc;
use wither::Model;

#[derive(Debug, Model, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{"name": 1}"#, options = r#"doc!{"unique": true}"#))]
pub struct Language {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
}

#[derive(Debug, Model, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{"language_name": 1}"#))]
pub struct Word {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub language_name: String,
    pub word: String,
}