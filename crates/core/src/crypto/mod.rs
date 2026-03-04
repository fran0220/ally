use aes::Aes256;
use aes_gcm::{
    AesGcm,
    aead::{AeadInPlace, KeyInit, generic_array::GenericArray, generic_array::typenum::U16},
};
use pbkdf2::pbkdf2_hmac_array;
use rand_core::{OsRng, RngCore};
use sha2::Sha256;
use thiserror::Error;

const API_KEY_SALT: &[u8] = b"waoowaoo-api-key-salt-v1";
const PBKDF2_ITERATIONS: u32 = 100_000;
const KEY_LENGTH: usize = 32;
const NONCE_LENGTH: usize = 16;
const TAG_LENGTH: usize = 16;

type Aes256Gcm16 = AesGcm<Aes256, U16>;

#[derive(Debug, Error)]
pub enum ApiKeyCryptoError {
    #[error("api encryption key is empty")]
    MissingEncryptionKey,
    #[error("api key plaintext is empty")]
    EmptyPlaintext,
    #[error("api key ciphertext is empty")]
    EmptyCiphertext,
    #[error("ciphertext format is invalid")]
    InvalidCiphertextFormat,
    #[error("ciphertext payload is invalid")]
    InvalidCiphertextPayload,
    #[error("failed to encrypt api key")]
    EncryptionFailed,
    #[error("failed to decrypt api key")]
    DecryptionFailed,
}

fn derive_encryption_key(secret: &str) -> Result<[u8; KEY_LENGTH], ApiKeyCryptoError> {
    let normalized = secret.trim();
    if normalized.is_empty() {
        return Err(ApiKeyCryptoError::MissingEncryptionKey);
    }

    Ok(pbkdf2_hmac_array::<Sha256, KEY_LENGTH>(
        normalized.as_bytes(),
        API_KEY_SALT,
        PBKDF2_ITERATIONS,
    ))
}

pub fn encrypt_api_key(
    plaintext: &str,
    encryption_secret: &str,
) -> Result<String, ApiKeyCryptoError> {
    if plaintext.trim().is_empty() {
        return Err(ApiKeyCryptoError::EmptyPlaintext);
    }

    let key = derive_encryption_key(encryption_secret)?;
    let cipher =
        Aes256Gcm16::new_from_slice(&key).map_err(|_| ApiKeyCryptoError::EncryptionFailed)?;

    let mut nonce = [0u8; NONCE_LENGTH];
    OsRng.fill_bytes(&mut nonce);

    let mut encrypted = plaintext.as_bytes().to_vec();
    let tag = cipher
        .encrypt_in_place_detached(GenericArray::from_slice(&nonce), b"", &mut encrypted)
        .map_err(|_| ApiKeyCryptoError::EncryptionFailed)?;

    Ok(format!(
        "{}:{}:{}",
        hex::encode(nonce),
        hex::encode(tag),
        hex::encode(encrypted)
    ))
}

pub fn decrypt_api_key(
    ciphertext: &str,
    encryption_secret: &str,
) -> Result<String, ApiKeyCryptoError> {
    let normalized = ciphertext.trim();
    if normalized.is_empty() {
        return Err(ApiKeyCryptoError::EmptyCiphertext);
    }

    let parts = normalized.split(':').collect::<Vec<_>>();
    if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
        return Err(ApiKeyCryptoError::InvalidCiphertextFormat);
    }

    let nonce = hex::decode(parts[0]).map_err(|_| ApiKeyCryptoError::InvalidCiphertextPayload)?;
    let tag = hex::decode(parts[1]).map_err(|_| ApiKeyCryptoError::InvalidCiphertextPayload)?;
    let mut encrypted =
        hex::decode(parts[2]).map_err(|_| ApiKeyCryptoError::InvalidCiphertextPayload)?;

    if nonce.len() != NONCE_LENGTH || tag.len() != TAG_LENGTH {
        return Err(ApiKeyCryptoError::InvalidCiphertextPayload);
    }

    let key = derive_encryption_key(encryption_secret)?;
    let cipher =
        Aes256Gcm16::new_from_slice(&key).map_err(|_| ApiKeyCryptoError::DecryptionFailed)?;

    cipher
        .decrypt_in_place_detached(
            GenericArray::from_slice(&nonce),
            b"",
            &mut encrypted,
            GenericArray::from_slice(&tag),
        )
        .map_err(|_| ApiKeyCryptoError::DecryptionFailed)?;

    String::from_utf8(encrypted).map_err(|_| ApiKeyCryptoError::InvalidCiphertextPayload)
}

#[cfg(test)]
mod tests {
    use super::{ApiKeyCryptoError, decrypt_api_key, encrypt_api_key};

    #[test]
    fn encrypt_and_decrypt_round_trip() {
        let ciphertext =
            encrypt_api_key("sk-test-key", "test-secret").expect("encryption should succeed");

        assert_ne!(ciphertext, "sk-test-key");
        assert_eq!(ciphertext.split(':').count(), 3);

        let plaintext =
            decrypt_api_key(&ciphertext, "test-secret").expect("decryption should succeed");
        assert_eq!(plaintext, "sk-test-key");
    }

    #[test]
    fn decrypt_rejects_invalid_format() {
        let err =
            decrypt_api_key("invalid", "test-secret").expect_err("invalid payload should fail");

        assert!(matches!(err, ApiKeyCryptoError::InvalidCiphertextFormat));
    }

    #[test]
    fn decrypt_fails_with_wrong_secret() {
        let ciphertext =
            encrypt_api_key("sk-test-key", "secret-a").expect("encryption should succeed");

        let err =
            decrypt_api_key(&ciphertext, "secret-b").expect_err("mismatched secret must fail");
        assert!(matches!(err, ApiKeyCryptoError::DecryptionFailed));
    }

    #[test]
    fn empty_secret_is_rejected() {
        let err = encrypt_api_key("sk-test-key", " ").expect_err("empty key should fail");
        assert!(matches!(err, ApiKeyCryptoError::MissingEncryptionKey));
    }
}
