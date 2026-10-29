//! Argon2id password hashing: m=64MB, t=3, p=4.

use argon2::password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::{Algorithm, Argon2, Params, Version};
use thiserror::Error;

use crate::{ARGON2_MEMORY_KB, ARGON2_PARALLELISM, ARGON2_TIME};

/// Errors from password hashing/verification.
#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("hash error: {0}")]
    Hash(#[from] argon2::Error),
    #[error("phc parse error: {0}")]
    Pw(#[from] argon2::password_hash::Error),
    #[error("phc parse error: {0}")]
    Phc(#[from] argon2::password_hash::phc::Error),
}

/// Build an Argon2id hasher with the spec parameters (m=64MB, t=3, p=4).
fn hasher() -> Result<Argon2<'static>, PasswordError> {
    let params = Params::new(ARGON2_MEMORY_KB, ARGON2_TIME, ARGON2_PARALLELISM, None)?;
    Ok(Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        params,
    ))
}

/// Hash a plaintext password (argon2id, random salt, PHC string output).
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    // Auto-generates a random salt; outputs a PHC string ($argon2id$v=19$...).
    let ph = hasher()?.hash_password(password.as_bytes())?;
    Ok(ph.to_string())
}

/// Verify a plaintext password against a stored PHC hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let parsed = PasswordHash::new(hash).map_err(PasswordError::Phc)?;
    // NOTE: verify uses the params embedded in the stored hash.
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("correct horse battery staple", &hash).unwrap());
        assert!(!verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn hash_is_salt_randomized() {
        let h1 = hash_password("same").unwrap();
        let h2 = hash_password("same").unwrap();
        assert_ne!(h1, h2, "different salts expected");
    }

    #[test]
    fn spec_params() {
        // m=64MB t=3 p=4
        assert_eq!(ARGON2_MEMORY_KB, 65536);
        assert_eq!(ARGON2_TIME, 3);
        assert_eq!(ARGON2_PARALLELISM, 4);
    }
}
