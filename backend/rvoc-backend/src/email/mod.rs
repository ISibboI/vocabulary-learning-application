//! Functions to send emails to users and admins.

use tracing::error;

use crate::error::{RVocError, RVocResult};

#[allow(unused)]
pub fn error_notification(error: &RVocError) -> RVocResult<()> {
    error!("E-mail error notifications not yet implemented, error is: {error}");
    Ok(())
}
