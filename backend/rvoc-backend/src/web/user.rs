#[derive(Debug)]
pub struct User {
    name: Username,
}

#[derive(Debug)]
pub struct Username {
    name: String,
}

impl Username {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn into_string(self) -> String {
        self.name
    }
}

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.name
    }
}
