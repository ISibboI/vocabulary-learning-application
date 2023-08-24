use super::hashed_password::HashedPassword;

pub struct User {
    name: Username,
    password: HashedPassword,
}

#[derive(Debug)]
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
