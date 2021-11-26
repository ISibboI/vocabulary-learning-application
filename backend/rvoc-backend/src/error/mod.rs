use std::net::AddrParseError;
use wither::mongodb::error::Error;
use wither::WitherError;

pub type RVocResult<T> = Result<T, RVocError>;

#[derive(Debug)]
pub enum RVocError {
    WitherError(WitherError),
    MongoDBError(wither::mongodb::error::Error),
    AddrParseError(AddrParseError),
}

impl From<WitherError> for RVocError {
    fn from(error: WitherError) -> Self {
        Self::WitherError(error)
    }
}

impl From<wither::mongodb::error::Error> for RVocError {
    fn from(error: Error) -> Self {
        Self::MongoDBError(error)
    }
}

impl From<AddrParseError> for RVocError {
    fn from(error: AddrParseError) -> Self {
        Self::AddrParseError(error)
    }
}
