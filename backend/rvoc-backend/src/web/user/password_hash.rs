use argon2::Argon2;
use argon2::PasswordHasher;
use password_hash::PasswordVerifier;
use password_hash::{rand_core::OsRng, SaltString};
use secstr::SecUtf8;

use crate::SecBytes;
use crate::{
    configuration::Configuration,
    error::{RVocError, RVocResult},
};

static HASH_ALGORITHM: argon2::Algorithm = argon2::Algorithm::Argon2id;
static HASH_ALGORITHM_VERSION: argon2::Version = argon2::Version::V0x13;

#[derive(Debug)]
pub struct PasswordHash {
    argon_hash: SecUtf8,
}

impl PasswordHash {
    pub fn new(
        plaintext_password: SecBytes,
        configuration: impl AsRef<Configuration>,
    ) -> RVocResult<Self> {
        let configuration = configuration.as_ref();

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
            HASH_ALGORITHM,
            HASH_ALGORITHM_VERSION,
            // the correctness of the parameters was checked when creating the configuration
            configuration.build_argon2_parameters()?,
        )
        .map_err(|error| RVocError::PasswordArgon2IdParameters {
            source: Box::new(error),
        })?;

        let argon_hash = argon2
            .hash_password(plaintext_password.unsecure(), &salt)
            .map_err(|error| RVocError::PasswordArgon2IdHash {
                source: Box::new(error),
            })?
            .to_string()
            .into();

        Ok(Self { argon_hash })
    }

    pub fn verify(
        &mut self,
        plaintext_password: SecBytes,
        configuration: impl AsRef<Configuration>,
    ) -> RVocResult<bool> {
        let parsed_hash = argon2::password_hash::PasswordHash::new(self.argon_hash.unsecure())
            .map_err(|error| RVocError::PasswordArgon2IdVerify {
                source: Box::new(error),
            })?;

        match Argon2::default().verify_password(plaintext_password.unsecure(), &parsed_hash) {
            Ok(()) => {
                if self.did_parameters_change(&parsed_hash, &configuration)? {
                    *self = Self::new(plaintext_password, configuration)?;
                }
                Ok(true)
            }
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(error) => Err(RVocError::PasswordArgon2IdVerify {
                source: Box::new(error),
            }),
        }
    }

    /// Check if the password hashing parameters are different from the ones used for this hash.
    fn did_parameters_change(
        &self,
        parsed_hash: &argon2::password_hash::PasswordHash<'_>,
        configuration: impl AsRef<Configuration>,
    ) -> RVocResult<bool> {
        let algorithm_identifier = parsed_hash.algorithm;
        let algorithm_version = parsed_hash.version;
        let algorithm_parameters = argon2::Params::try_from(parsed_hash).map_err(|error| {
            RVocError::PasswordArgon2IdRehash {
                source: Box::new(error),
            }
        })?;

        Ok(algorithm_identifier != HASH_ALGORITHM.ident()
            || algorithm_version != Some(HASH_ALGORITHM_VERSION.into())
            || algorithm_parameters != configuration.as_ref().build_argon2_parameters()?)
    }
}

impl From<PasswordHash> for String {
    fn from(value: PasswordHash) -> Self {
        value.argon_hash.into_unsecure()
    }
}
