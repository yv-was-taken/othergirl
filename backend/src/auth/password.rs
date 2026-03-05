use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use argon2::password_hash::rand_core::OsRng;

use crate::error::{AppError, AppResult};

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);

    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("password hashing failed: {e}")))?
        .to_string();

    Ok(hash)
}

pub fn verify_password(hash: &str, password: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(hash).map_err(|e| AppError::Internal(format!("password hash parsing failed: {e}")))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_password_matches() {
        let hash = hash_password("my-secret-password").unwrap();
        assert!(verify_password(&hash, "my-secret-password").unwrap());
    }

    #[test]
    fn wrong_password_fails() {
        let hash = hash_password("my-secret-password").unwrap();
        assert!(!verify_password(&hash, "wrong-password").unwrap());
    }

    #[test]
    fn hash_format_is_valid_argon2() {
        let hash = hash_password("test-password").unwrap();
        assert!(hash.starts_with("$argon2"), "hash should start with $argon2, got: {hash}");
        // Verify the hash can be parsed back
        PasswordHash::new(&hash).expect("hash should be a valid PHC string");
    }

    #[test]
    fn different_passwords_produce_different_hashes() {
        let hash1 = hash_password("password-one").unwrap();
        let hash2 = hash_password("password-two").unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn same_password_produces_different_hashes_due_to_salt() {
        let hash1 = hash_password("same-password").unwrap();
        let hash2 = hash_password("same-password").unwrap();
        assert_ne!(hash1, hash2, "hashes should differ due to random salt");
        // But both should verify
        assert!(verify_password(&hash1, "same-password").unwrap());
        assert!(verify_password(&hash2, "same-password").unwrap());
    }

    #[test]
    fn empty_password_can_be_hashed_and_verified() {
        let hash = hash_password("").unwrap();
        assert!(verify_password(&hash, "").unwrap());
        assert!(!verify_password(&hash, "non-empty").unwrap());
    }

    #[test]
    fn invalid_hash_string_returns_error() {
        let result = verify_password("not-a-valid-hash", "password");
        assert!(result.is_err());
    }
}
