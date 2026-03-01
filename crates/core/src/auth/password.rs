use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand_core::OsRng;

use crate::errors::AppError;

pub fn hash_password(plain: &str) -> Result<String, AppError> {
    if plain.len() < 6 {
        return Err(AppError::invalid_params(
            "password must be at least 6 chars",
        ));
    }
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    argon
        .hash_password(plain.as_bytes(), &salt)
        .map(|hashed| hashed.to_string())
        .map_err(|err| AppError::internal(format!("failed to hash password: {err}")))
}

pub fn verify_password(plain: &str, hashed: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(hashed)
        .map_err(|err| AppError::internal(format!("invalid password hash: {err}")))?;
    let argon = Argon2::default();
    Ok(argon.verify_password(plain.as_bytes(), &parsed).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_and_verify() {
        let hash = hash_password("123456").expect("hash should be created");
        assert!(verify_password("123456", &hash).expect("verify should run"));
        assert!(!verify_password("654321", &hash).expect("verify should run"));
    }
}
