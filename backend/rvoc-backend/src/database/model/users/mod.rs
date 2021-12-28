use crate::error::RVocError;
use crate::RVocResult;
use argon2::Argon2;
use password_hash::{PasswordHash, SaltString};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use wither::bson::oid::ObjectId;
use wither::mongodb::bson::doc;
use wither::Model;

/// A user of the application.
#[derive(Debug, Model, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{"name": 1}"#, options = r#"doc!{"unique": true}"#))]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    /// The login name of the user.
    pub login_name: String,
    /// The email address of the user.
    pub email: String,
    /// The password of the user.
    pub password: HashedPassword,
}

/// A password in hashed form.
#[derive(Debug, Serialize, Deserialize)]
pub struct HashedPassword {
    hashed_password: String,
}

impl HashedPassword {
    pub fn new(plain_text_password: &str) -> RVocResult<Self> {
        if plain_text_password.len() > 1024 {
            return Err(RVocError::PasswordTooLong);
        }

        let salt = SaltString::generate(&mut OsRng);

        Ok(Self {
            hashed_password: PasswordHash::generate(
                Argon2::default(),
                plain_text_password,
                salt.as_str(),
            )?
            .to_string(),
        })
    }
}
