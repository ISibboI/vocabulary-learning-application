use serde::{Deserialize, Serialize};
use wither::bson::oid::ObjectId;
use wither::mongodb::bson::doc;
use wither::Model;

/// A language.
#[derive(Debug, Model, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{"name": 1}"#, options = r#"doc!{"unique": true}"#))]
pub struct Language {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    /// The name of the language.
    pub name: String,
}

/// A single word in a language.
#[derive(Debug, Model, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{"language_name": 1}"#))]
pub struct Word {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    /// The name of the language this word belongs to.
    pub language_name: String,
    /// The word's textual representation.
    pub word: String,
}
