use secstr::SecVec;

type SecBytes = SecVec<u8>;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct CreateAccount {
    pub name: String,
    pub password: SecBytes,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Login {
    pub name: String,
    pub password: SecBytes,
}

#[cfg(test)]
mod tests {
    use crate::CreateAccount;

    #[test]
    fn test_serde_create_account() {
        let create_account = CreateAccount {
            name: "anne".to_owned(),
            password: "frank".to_owned().into(),
        };

        let json = serde_json::to_string_pretty(&create_account).unwrap();
        println!("json = {json}");

        let create_account_serde: CreateAccount = serde_json::from_str(&json).unwrap();

        assert_eq!(create_account, create_account_serde);
    }
}
