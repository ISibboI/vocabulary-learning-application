use secstr::SecVec;

type SecBytes = SecVec<u8>;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CreateAccount {
    pub name: String,
    pub password: SecBytes,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_serde_create_account() {}
}
