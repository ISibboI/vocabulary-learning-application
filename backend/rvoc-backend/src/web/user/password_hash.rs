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

#[derive(Clone, Debug)]
pub struct PasswordHash {
    argon_hash: Option<SecUtf8>,
}

#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub struct VerifyPasswordResult {
    /// True if the password matches the hash.
    pub matches: bool,

    /// True if the password hash was modified and needs to be written to the database.
    pub modified: bool,
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

        let argon2 = Self::build_argon2(configuration)?;

        let argon_hash = Some(
            argon2
                .hash_password(plaintext_password.unsecure(), &salt)
                .map_err(|error| RVocError::PasswordArgon2IdHash {
                    source: Box::new(error),
                })?
                .to_string()
                .into(),
        );

        Ok(Self { argon_hash })
    }

    pub fn verify(
        &mut self,
        plaintext_password: SecBytes,
        configuration: impl AsRef<Configuration>,
    ) -> RVocResult<VerifyPasswordResult> {
        let Some(argon_hash) = &self.argon_hash else {
            return Err(RVocError::PasswordArgon2IdVerify {
                source: Box::new(password_hash::Error::Password),
            });
        };

        let configuration = configuration.as_ref();
        let parsed_hash =
            argon2::password_hash::PasswordHash::new(argon_hash.unsecure()).map_err(|error| {
                RVocError::PasswordArgon2IdVerify {
                    source: Box::new(error),
                }
            })?;
        let argon2 = Self::build_argon2_from_parameters(
            argon2::Params::try_from(&parsed_hash).map_err(|error| {
                RVocError::PasswordArgon2IdVerify {
                    source: Box::new(error),
                }
            })?,
            configuration,
        )?;

        match argon2.verify_password(plaintext_password.unsecure(), &parsed_hash) {
            Ok(()) => {
                let modified = if self.did_parameters_change(&parsed_hash, configuration)? {
                    *self = Self::new(plaintext_password, configuration)?;
                    true
                } else {
                    false
                };
                Ok(VerifyPasswordResult {
                    matches: true,
                    modified,
                })
            }
            Err(argon2::password_hash::Error::Password) => Ok(VerifyPasswordResult {
                matches: false,
                modified: false,
            }),
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
        let configuration = configuration.as_ref();
        let algorithm_identifier = parsed_hash.algorithm;
        let algorithm_version = parsed_hash.version;
        let algorithm_parameters = argon2::Params::try_from(parsed_hash).map_err(|error| {
            RVocError::PasswordArgon2IdRehash {
                source: Box::new(error),
            }
        })?;

        let configured_parameters = configuration.build_argon2_parameters()?;

        Ok(algorithm_identifier != HASH_ALGORITHM.ident()
            || algorithm_version != Some(HASH_ALGORITHM_VERSION.into())
            || algorithm_parameters.m_cost() != configured_parameters.m_cost()
            || algorithm_parameters.t_cost() != configured_parameters.t_cost()
            || algorithm_parameters.p_cost() != configured_parameters.p_cost())
    }

    fn build_argon2_from_parameters(
        parameters: argon2::Params,
        configuration: &Configuration,
    ) -> RVocResult<Argon2<'_>> {
        Argon2::new_with_secret(
            configuration.password_pepper.unsecure(),
            HASH_ALGORITHM,
            HASH_ALGORITHM_VERSION,
            // the correctness of the parameters was checked when creating the configuration
            parameters,
        )
        .map_err(|error| RVocError::PasswordArgon2IdParameters {
            source: Box::new(error),
        })
    }

    fn build_argon2(configuration: &Configuration) -> RVocResult<Argon2<'_>> {
        Self::build_argon2_from_parameters(configuration.build_argon2_parameters()?, configuration)
    }
}

impl From<PasswordHash> for Option<String> {
    fn from(value: PasswordHash) -> Self {
        value.argon_hash.map(SecUtf8::into_unsecure)
    }
}

impl From<Option<String>> for PasswordHash {
    fn from(value: Option<String>) -> Self {
        Self {
            argon_hash: value.map(Into::into),
        }
    }
}

impl From<String> for PasswordHash {
    fn from(value: String) -> Self {
        Self {
            argon_hash: Some(value.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        configuration::Configuration,
        web::user::password_hash::{VerifyPasswordResult, HASH_ALGORITHM, HASH_ALGORITHM_VERSION},
        SecBytes,
    };

    use super::PasswordHash;

    #[test]
    fn test_password_check() {
        let configuration = Configuration::test_configuration();

        println!("Hash algo: {}", HASH_ALGORITHM.ident());
        println!("Hash algo version: {}", u32::from(HASH_ALGORITHM_VERSION));
        println!(
            "Hash algo parameters: {:?}",
            configuration.build_argon2_parameters().unwrap()
        );

        let password = SecBytes::from("mypassword");
        let mut password_hash = PasswordHash::new(password.clone(), &configuration).unwrap();

        let verify_password_result = password_hash.verify(password.clone(), &configuration);
        assert!(
            verify_password_result.is_ok(),
            "password hash result: {verify_password_result:?}"
        );
        assert_eq!(
            verify_password_result.unwrap(),
            VerifyPasswordResult {
                matches: true,
                modified: false,
            }
        );

        // convert to string and back
        let password_hash_string = Option::<String>::from(password_hash).unwrap();
        let mut password_hash = PasswordHash::from(Some(password_hash_string));
        let verify_password_result = password_hash.verify(password.clone(), &configuration);
        assert!(
            verify_password_result.is_ok(),
            "password hash result: {verify_password_result:?}"
        );
        assert_eq!(
            verify_password_result.unwrap(),
            VerifyPasswordResult {
                matches: true,
                modified: false,
            }
        );
    }
}
