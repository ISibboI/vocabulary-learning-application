use crate::api_server::SignupCommand;
use crate::configuration::Configuration;
use crate::error::{RVocError, RVocResult};
use argon2::Argon2;
use chrono::{Duration, Utc};
use log::info;
use password_hash::{PasswordHash, SaltString};
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use wither::bson::oid::ObjectId;
use wither::mongodb::bson::doc;
use wither::mongodb::Database;
use wither::{bson, Model, WitherError};

/// A user of the application.
#[derive(Debug, Model, Serialize, Deserialize)]
#[model(
    index(keys = r#"doc!{"login_name": 1}"#, options = r#"doc!{"unique": true}"#),
    index(keys = r#"doc!{"email": 1}"#, options = r#"doc!{"unique": true}"#),
    index(
        keys = r#"doc!{"sessions.session_id.session_id": 1}"#,
        options = r#"doc!{"unique": true, "sparse": true}"#
    )
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
    sessions: Vec<Session>,
}

impl User {
    pub async fn create(
        signup_command: &SignupCommand,
        database: &Database,
        configuration: &Configuration,
    ) -> RVocResult<Self> {
        let mut user = Self {
            id: None,
            login_name: signup_command.login_name.clone(),
            email: signup_command.email.clone(),
            password: HashedPassword::new(&signup_command.password, configuration).await?,
            sessions: Vec::new(),
        };
        user.save(database, None).await?;
        Ok(user)
    }

    pub async fn find_by_session_id(
        database: &Database,
        session_id: &SessionId,
    ) -> RVocResult<(Self, Session)> {
        // session ids have a unique index, so there is never more than one user with a given session id.
        let user = Self::find_one(
            database,
            doc! {"sessions.session_id.session_id": session_id.to_string()},
            None,
        )
        .await
        .map_err(|error| error)?
        .ok_or_else(|| RVocError::SessionIdNotFound(session_id.clone()))?;
        let user = if user
            .find_session_by_session_id(session_id)
            .unwrap()
            .is_outdated()
        {
            user.delete_outdated_sessions(database).await?
        } else {
            user
        };
        let session = user
            .find_session_by_session_id(session_id)
            .ok_or_else(|| RVocError::SessionIdNotFound(session_id.clone()))?
            .clone();
        Ok((user, session))
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

    pub fn find_session_by_session_id(&self, session_id: &SessionId) -> Option<&Session> {
        self.sessions
            .iter()
            .find(|session| &session.session_id == session_id)
    }

    pub async fn create_session(
        self,
        database: &Database,
        configuration: &Configuration,
    ) -> RVocResult<(Self, Session)> {
        static ATTEMPTS: usize = 100;
        let mut updated_user = self;
        let login_name = updated_user.login_name.clone();
        for _ in 0..ATTEMPTS {
            let session = Session::new(configuration).await;
            let mut retry = false;
            updated_user = match updated_user
                .update(
                    database,
                    None,
                    doc! {"$push": doc! {"sessions": session.to_doc()?}},
                    None,
                )
                .await
            {
                Ok(user) => user,
                Err(WitherError::Mongo(error @ wither::mongodb::error::Error { .. })) => {
                    let kind = error.kind.clone();
                    match kind.borrow() {
                        wither::mongodb::error::ErrorKind::Command(
                            wither::mongodb::error::CommandError {
                                code, code_name, ..
                            },
                        ) => {
                            // Check if the session id already exists, 11000 is the error code for a duplicate key.
                            // This error occurs because we have a unique index on the session ids.
                            if *code == 11000 {
                                assert_eq!(code_name, "DuplicateKey");
                                // We accidentally randomly chose an existing session id, so we just try again.
                                retry = true;
                                User::find_by_login_name(database, &login_name).await?
                            } else {
                                return Err(error.into());
                            }
                        }
                        _ => return Err(error.into()),
                    }
                }
                Err(error) => return Err(error.into()),
            };
            if retry {
                continue;
            }
            updated_user.sessions.push(session.clone());
            return Ok((updated_user, session));
        }

        Err(RVocError::NoFreeSessionId { attempts: ATTEMPTS })
    }

    pub async fn update_session(
        self,
        session: Session,
        database: &Database,
        configuration: &Configuration,
    ) -> RVocResult<(Self, Session)> {
        let session = session.update(configuration);
        let mut updated_self = self
            .update(
                database,
                Some(doc! {"sessions.session_id.session_id": session.session_id().to_string()}),
                doc! {"$set": {"sessions.$.expires": session.expires}},
                None,
            )
            .await
            .map_err(RVocError::CannotUpdateSessionExpiry)?;
        let session = updated_self
            .find_session_by_session_id(&session.session_id)
            .unwrap()
            .clone();
        updated_self.sessions = updated_self
            .sessions
            .into_iter()
            .map(|iter_session| {
                if iter_session.session_id == session.session_id {
                    session.clone()
                } else {
                    iter_session
                }
            })
            .collect();
        Ok((updated_self, session))
    }

    pub async fn delete_outdated_sessions(self, database: &Database) -> RVocResult<Self> {
        let now = bson::DateTime::now();
        let result = self
            .update(
                database,
                None,
                doc! {"$pull": {"sessions": {"expires": {"$lt": now}}}},
                None,
            )
            .await
            .map_err(RVocError::CannotDeleteExpiredSessions)?;
        let result = Self::find_by_login_name(database, result.login_name).await?;
        Ok(result)
    }

    pub async fn delete_session(self, session: &Session, database: &Database) -> RVocResult<Self> {
        info!("Deleting session: {session:?}");
        let result = self
            .update(database, None, doc! {"$pull": {"sessions": {"session_id.session_id": session.session_id().to_string()}}}, None)
            .await
            .map_err(|error| {info!("Delete session error: {error:?}");RVocError::CannotDeleteSession})?;
        let result = Self::find_by_login_name(database, result.login_name).await?;
        Ok(result)
    }

    pub async fn delete_other_sessions(
        self,
        session: &Session,
        database: &Database,
    ) -> RVocResult<Self> {
        let result = self
            .update(database, None, doc! {"$pull": {"sessions.session_id.session_id": {"$ne": session.session_id().to_string()}}}, None)
            .await
            .map_err(|_| RVocError::CannotDeleteOtherSessions)?;
        let result = Self::find_by_login_name(database, result.login_name).await?;
        Ok(result)
    }

    pub async fn delete_all_sessions(self, database: &Database) -> RVocResult<Self> {
        let result = self
            .update(database, None, doc! {"set": {"sessions": []}}, None)
            .await
            .map_err(|_| RVocError::CannotDeleteAllSessions)?;
        let result = Self::find_by_login_name(database, result.login_name).await?;
        Ok(result)
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

/// A session with a limited lifetime, identified by an id.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Session {
    session_id: SessionId,
    expires: bson::DateTime,
}

impl Session {
    pub async fn new(configuration: &Configuration) -> Self {
        Self {
            session_id: SessionId::new(configuration).await,
            expires: bson::DateTime::from_chrono(
                Utc::now() + Duration::seconds(configuration.session_cookie_max_age_seconds),
            ),
        }
    }

    pub fn update(mut self, configuration: &Configuration) -> Self {
        self.expires = bson::DateTime::from_chrono(
            Utc::now() + Duration::seconds(configuration.session_cookie_max_age_seconds),
        );
        self
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn expires(&self) -> &bson::DateTime {
        &self.expires
    }

    pub fn to_doc(&self) -> bson::ser::Result<bson::Document> {
        bson::to_document(self)
    }

    pub fn is_outdated(&self) -> bool {
        self.expires < bson::DateTime::now()
    }
}

/// A session id.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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
