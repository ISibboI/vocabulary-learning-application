use crate::error::RVocResult;

pub mod hashed_password;
pub mod model;

pub async fn create_account() -> RVocResult<()> {
    Ok(())
}
