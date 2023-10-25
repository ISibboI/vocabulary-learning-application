use diesel::prelude::Insertable;

use self::{password_hash::PasswordHash, username::Username};

pub mod password_hash;
pub mod username;

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
