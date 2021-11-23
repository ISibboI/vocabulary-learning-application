use wither::mongodb::error::Error;
use wither::WitherError;

pub type RVocResult<T> = Result<T, RVocError>;

#[derive(Debug)]
pub enum RVocError {
    WitherError(WitherError),
    MongoDBError(wither::mongodb::error::Error),
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