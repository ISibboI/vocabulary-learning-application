use crate::error::RVocError;
use crate::{Configuration, RVocResult};
use argon2::Argon2;
use password_hash::{PasswordHash, SaltString};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use wither::bson::oid::ObjectId;
use wither::mongodb::bson::doc;
use wither::Model;

/// A user of the application.
#[derive(Debug, Model, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{"login_name": 1}"#, options = r#"doc!{"unique": true}"#))]
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

// Temporary silence of unused warnings.
#[allow(unused)]
impl HashedPassword {
    /// Create a hashed representation of the given password.
    /// The salt for the hash is generated with [OsRng](rand_core::OsRng).
    pub fn new(plain_text_password: &str, configuration: &Configuration) -> RVocResult<Self> {
        if plain_text_password.len() > configuration.max_password_bytes {
            return Err(RVocError::PasswordTooLong {
                password_bytes: plain_text_password.len(),
                max_bytes: configuration.max_password_bytes,
            });
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

    /// Check if the given password matches the hashed representation.
    pub fn check(
        &self,
        plain_text_password: &str,
        configuration: &Configuration,
    ) -> RVocResult<bool> {
        if plain_text_password.len() > configuration.max_password_bytes {
            return Err(RVocError::PasswordTooLong {
                password_bytes: plain_text_password.len(),
                max_bytes: configuration.max_password_bytes,
            });
        }

        let password_hash = PasswordHash::new(&self.hashed_password)?;
        Ok(password_hash
            .verify_password(&[&Argon2::default()], plain_text_password)
            .is_ok())
    }
}

#[cfg(test)]
mod tests {
    use crate::database::model::users::HashedPassword;
    use crate::Configuration;
    use clap::Parser;
    use std::ffi::OsString;
    use std::iter;

    #[test]
    fn test_password_check() {
        let configuration = Configuration::parse_from(iter::empty::<OsString>());
        let password = "abc123";
        let hashed_password = HashedPassword::new(password, &configuration).unwrap();
        assert!(hashed_password.check(password, &configuration).unwrap());
        assert!(!hashed_password
            .check("wrong password", &configuration)
            .unwrap());
    }
}
