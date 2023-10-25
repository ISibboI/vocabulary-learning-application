use crate::{configuration::Configuration, error::RVocResult};

#[derive(Debug, Clone)]
pub struct Username {
    name: String,
}

impl Username {
    pub fn new(name: String, configuration: impl AsRef<Configuration>) -> RVocResult<Self> {
        configuration.as_ref().verify_username_length(&name)?;

        Ok(Self { name })
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
