use crate::error::{RVocError, RVocResult};
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

/// The maximum length of a password in bytes.
pub static MAXIMUM_PASSWORD_BYTES: usize = 1024;

// Temporary silence of unused warnings.
#[allow(unused)]
impl HashedPassword {
    /// Create a hashed representation of the given password.
    /// The salt for the hash is generated with [OsRng](rand_core::OsRng).
    pub fn new(plain_text_password: &str) -> RVocResult<Self> {
        if plain_text_password.len() > MAXIMUM_PASSWORD_BYTES {
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

    /// Check if the given password matches the hashed representation.
    pub fn check(&self, plain_text_password: &str) -> RVocResult<bool> {
        if plain_text_password.len() > MAXIMUM_PASSWORD_BYTES {
            return Err(RVocError::PasswordTooLong);
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

    #[test]
    fn test_password_check() {
        let password = "abc123";
        let hashed_password = HashedPassword::new(password).unwrap();
        assert!(hashed_password.check(password).unwrap());
        assert!(!hashed_password.check("wrong password").unwrap());
    }
}
