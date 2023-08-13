use crate::{
    configuration::Configuration, error::RVocResult, update_wiktionary::run_update_wiktionary,
};

pub async fn update_wiktionary(configuration: &Configuration) -> RVocResult<()> {
    run_update_wiktionary(configuration).await
}
