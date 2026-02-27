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
