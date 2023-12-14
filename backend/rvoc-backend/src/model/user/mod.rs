use chrono::{DateTime, Utc};
use diesel::{deserialize::Queryable, prelude::Insertable, AsChangeset, Identifiable, Selectable};
use tracing::trace;

use crate::configuration::Configuration;

use self::{password_hash::PasswordHash, username::Username};

pub mod password_hash;
pub mod username;

#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = crate::database::schema::users)]
#[diesel(primary_key(name))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(treat_none_as_default_value = false)]
pub struct NewUser {
    #[diesel(serialize_as = String)]
    pub name: Username,
    #[diesel(serialize_as = Option<String>)]
    pub password_hash: PasswordHash,
}

#[derive(Insertable, Clone, Debug, Selectable, Queryable, Identifiable, AsChangeset)]
#[diesel(table_name = crate::database::schema::users)]
#[diesel(primary_key(name))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(treat_none_as_default_value = false)]
pub struct UserLoginInfo {
    #[diesel(serialize_as = String, deserialize_as = String)]
    pub name: Username,
    #[diesel(serialize_as = Option<String>, deserialize_as = Option<String>)]
    pub password_hash: PasswordHash,
    login_attempt_count: i32,
    failed_login_attempt_count: i32,
    next_login_attempt_count_reset: DateTime<Utc>,
}

impl NewUser {
    pub fn new(name: Username, password_hash: PasswordHash) -> Self {
        Self {
            name,
            password_hash,
        }
    }
}

impl UserLoginInfo {
    /// Checks if a login attempt can be made and increments the number of login attempts if yes.
    /// Returns `true` if a login attempt can be made.
    pub fn try_login_attempt(&mut self, now: DateTime<Utc>, configuration: &Configuration) -> bool {
        trace!("Trying login attempt with {self:?}");
        if self.can_attempt_to_login(now, configuration) {
            if self.login_attempt_count == 0 && self.failed_login_attempt_count == 0 {
                self.next_login_attempt_count_reset =
                    now + configuration.login_attempt_counting_interval;
            }
            self.login_attempt_count += 1;
            true
        } else {
            false
        }
    }

    /// Record a failed login attempt.
    pub fn fail_login_attempt(&mut self) {
        assert!(self.login_attempt_count > 0);
        self.failed_login_attempt_count += 1;
    }

    /// Returns `true` if it is currently possible to attempt a login.
    fn can_attempt_to_login(&mut self, now: DateTime<Utc>, configuration: &Configuration) -> bool {
        if now >= self.next_login_attempt_count_reset {
            self.login_attempt_count = 0;
            self.failed_login_attempt_count = 0;
            true
        } else {
            self.login_attempt_count < configuration.max_login_attempts_per_interval
                && self.failed_login_attempt_count
                    < configuration.max_failed_login_attempts_per_interval
        }
    }
}
