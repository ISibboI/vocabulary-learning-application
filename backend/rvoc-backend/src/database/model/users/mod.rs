use crate::configuration::Configuration;
use crate::error::{RVocError, RVocResult};
use argon2::Argon2;
use password_hash::{PasswordHash, SaltString};
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use wither::bson::oid::ObjectId;
use wither::mongodb::bson::doc;
use wither::mongodb::Database;
use wither::Model;

/// A user of the application.
#[derive(Debug, Model, Serialize, Deserialize)]
#[model(
    index(keys = r#"doc!{"login_name": 1}"#, options = r#"doc!{"unique": true}"#),
    index(keys = r#"doc!{"sessions": 1}"#, options = r#"doc!{"unique": true}"#)
)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    /// The login name of the user.
    pub login_name: String,
    /// The email address of the user.
    pub email: String,
    /// The password of the user.
    pub password: HashedPassword,
    /// The session ids used by a user.
    pub sessions: Vec<SessionId>,
}

impl User {
    pub async fn find_by_session_id(
        database: &Database,
        session_id: &SessionId,
    ) -> RVocResult<Self> {
        // session ids have a unique index, so there is never more than one user with a given session id.
        Self::find_one(database, doc! {"sessions": session_id.to_string()}, None)
            .await?
            .ok_or_else(|| RVocError::SessionIdNotFound(session_id.clone()))
    }

    pub async fn find_by_login_name(
        database: &Database,
        login_name: impl AsRef<str>,
    ) -> RVocResult<Self> {
        // login names have a unique index, so there is never more than one user with a given session id.
        Self::find_one(database, doc! {"login_name": login_name.as_ref()}, None)
            .await?
            .ok_or_else(|| RVocError::LoginNameNotFound(login_name.as_ref().to_string()))
    }

    pub async fn create_session(
        mut self,
        database: &Database,
        configuration: &Configuration,
    ) -> RVocResult<(Self, SessionId)> {
        let session_id = SessionId::new(configuration).await;
        self.sessions.push(session_id.clone());
        Ok((
            self.update(
                database,
                None,
                doc! {"$push": doc! {"sessions": session_id.to_string()}},
                None,
            )
            .await?,
            session_id,
        ))
    }
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
    pub async fn new(plain_text_password: &str, configuration: &Configuration) -> RVocResult<Self> {
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
    pub async fn check(
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

/// A session id.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionId {
    session_id: String,
}

impl ToString for SessionId {
    fn to_string(&self) -> String {
        self.session_id.clone()
    }
}

impl SessionId {
    pub fn try_from_string(session_id: String, configuration: &Configuration) -> RVocResult<Self> {
        if session_id.len() != configuration.session_id_length {
            Err(RVocError::WrongSessionIdLength {
                given_session_id_length: session_id.len(),
                configured_session_id_length: configuration.session_id_length,
            })
        } else if !Self::is_valid_session_id(&session_id) {
            Err(RVocError::InvalidSessionId(session_id))
        } else {
            Ok(Self { session_id })
        }
    }

    #[allow(unused)]
    pub async fn new(configuration: &Configuration) -> Self {
        let mut rng = OsRng::default();
        let mut session_id = String::with_capacity(configuration.session_id_length);
        // This is the same as for base64
        static CHAR_TABLE: [char; 64] = [
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q',
            'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h',
            'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y',
            'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '+', '/',
        ];
        for _ in 0..configuration.session_id_length {
            session_id.push(CHAR_TABLE.choose(&mut rng).cloned().unwrap());
        }
        Self { session_id }
    }

    pub fn is_valid_session_id(session_id: impl AsRef<str>) -> bool {
        session_id.as_ref().chars().all(|c| {
            ('A'..='Z').contains(&c)
                || ('a'..='z').contains(&c)
                || ('0'..='9').contains(&c)
                || c == '+'
                || c == '/'
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::configuration::Configuration;
    use crate::database::model::users::HashedPassword;
    use std::ffi::OsString;
    use std::iter;

    #[tokio::test]
    async fn test_password_check() {
        let configuration = Configuration::parse_from(iter::empty::<OsString>());
        let password = "abc123";
        let hashed_password = HashedPassword::new(password, &configuration).await.unwrap();
        assert!(hashed_password
            .check(password, &configuration)
            .await
            .unwrap());
        assert!(!hashed_password
            .check("wrong password", &configuration)
            .await
            .unwrap());
    }
}
