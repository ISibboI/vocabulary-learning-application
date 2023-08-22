use argon2::Argon2;
use argon2::PasswordHasher;
use password_hash::{rand_core::OsRng, SaltString};
use secstr::SecStr;

use crate::{
    configuration::Configuration,
    error::{RVocError, RVocResult},
};

#[derive(Debug)]
pub struct HashedPassword {
    argon_hash: SecStr,
}

impl HashedPassword {
    pub fn new(plaintext_password: SecStr, configuration: &Configuration) -> RVocResult<Self> {
        // the password length should be checked at the point where we have the password as string.
        let plaintext_password_length = plaintext_password.unsecure().len();
        assert!(
            plaintext_password_length >= configuration.minimum_password_length
        // times 4 because this is the length in bytes, and not in unicode code points
            && plaintext_password_length <= configuration.maximum_password_length * 4
        );

        let salt = SaltString::generate(&mut OsRng);

        let argon2 = Argon2::new_with_secret(
            configuration.password_pepper.unsecure(),
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            // the correctness of the parameters was checked when creating the configuration
            configuration.build_argon2_parameters()?,
        )
        .map_err(|error| RVocError::PasswordArgon2IdParameters { error })?;

        let argon_hash = argon2
            .hash_password(plaintext_password.unsecure(), &salt)
            .map_err(|error| RVocError::PasswordArgon2IdHash {
                source: Box::new(error),
            })?
            .to_string()
            .into();

        Ok(Self { argon_hash })
    }
}
