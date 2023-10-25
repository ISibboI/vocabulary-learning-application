use diesel::prelude::Insertable;

use self::password_hash::PasswordHash;

pub mod password_hash;

#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = crate::database::schema::users)]
#[diesel(primary_key(name))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(treat_none_as_default_value = false)]
pub struct User {
    #[diesel(serialize_as = String)]
    pub name: Username,
    #[diesel(serialize_as = Option<String>)]
    pub password_hash: PasswordHash,
}

#[derive(Debug, Clone)]
pub struct Username {
    name: String,
}

impl Username {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl From<Username> for String {
    fn from(value: Username) -> Self {
        value.name
    }
}
