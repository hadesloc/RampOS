use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, AeadCore, Nonce,
};
use ramp_common::Result;

/// Service for encrypting/decrypting secrets at rest using AES-256-GCM.
///
/// The master key is loaded from the `ENCRYPTION_MASTER_KEY` environment variable
/// and must be exactly 32 bytes (hex-encoded = 64 hex chars).
#[derive(Clone)]
pub struct CryptoService {
    cipher: Aes256Gcm,
}

impl CryptoService {
    /// Create a new CryptoService from a 32-byte key.
    pub fn from_key(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new_from_slice(key)
            .expect("AES-256-GCM key must be 32 bytes");
        Self { cipher }
    }

    /// Create a new CryptoService from the `ENCRYPTION_MASTER_KEY` env var (hex-encoded).
    pub fn from_env() -> std::result::Result<Self, ramp_common::Error> {
        let hex_key = std::env::var("ENCRYPTION_MASTER_KEY").map_err(|_| {
            ramp_common::Error::Encryption(
                "ENCRYPTION_MASTER_KEY environment variable not set".to_string(),
            )
        })?;

        let key_bytes = hex::decode(&hex_key).map_err(|e| {
            ramp_common::Error::Encryption(format!(
                "ENCRYPTION_MASTER_KEY is not valid hex: {}",
                e
            ))
        })?;

        if key_bytes.len() != 32 {
            return Err(ramp_common::Error::Encryption(format!(
                "ENCRYPTION_MASTER_KEY must be 32 bytes (64 hex chars), got {} bytes",
                key_bytes.len()
            )));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(Self::from_key(&key))
    }

    /// Encrypt plaintext using AES-256-GCM with a random 96-bit nonce.
    /// Returns (nonce, ciphertext).
    pub fn encrypt_secret(&self, plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| ramp_common::Error::Encryption(format!("Encryption failed: {}", e)))?;
        Ok((nonce.to_vec(), ciphertext))
    }

    /// Decrypt ciphertext using AES-256-GCM.
    pub fn decrypt_secret(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        if nonce.len() != 12 {
            return Err(ramp_common::Error::Encryption(format!(
                "Nonce must be 12 bytes, got {}",
                nonce.len()
            )));
        }
        let nonce = Nonce::from_slice(nonce);
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| ramp_common::Error::Encryption(format!("Decryption failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        // Deterministic test key
        let mut key = [0u8; 32];
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = i as u8;
        }
        key
    }

    #[test]
    fn test_encrypt_decrypt_round_trip() {
        let svc = CryptoService::from_key(&test_key());
        let plaintext = b"ramp_secret_abc123def456";

        let (nonce, ciphertext) = svc.encrypt_secret(plaintext).unwrap();

        assert_eq!(nonce.len(), 12);
        assert_ne!(ciphertext, plaintext);

        let decrypted = svc.decrypt_secret(&nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let svc1 = CryptoService::from_key(&test_key());
        let plaintext = b"sensitive_data";

        let (nonce, ciphertext) = svc1.encrypt_secret(plaintext).unwrap();

        // Different key
        let mut wrong_key = [0u8; 32];
        wrong_key[0] = 0xFF;
        let svc2 = CryptoService::from_key(&wrong_key);

        let result = svc2.decrypt_secret(&nonce, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_nonce_fails() {
        let svc = CryptoService::from_key(&test_key());
        let plaintext = b"sensitive_data";

        let (_nonce, ciphertext) = svc.encrypt_secret(plaintext).unwrap();

        // Wrong nonce
        let wrong_nonce = vec![0xFF; 12];
        let result = svc.decrypt_secret(&wrong_nonce, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_nonce_length() {
        let svc = CryptoService::from_key(&test_key());
        let result = svc.decrypt_secret(&[0u8; 8], &[0u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let svc = CryptoService::from_key(&test_key());
        let plaintext = b"";

        let (nonce, ciphertext) = svc.encrypt_secret(plaintext).unwrap();
        let decrypted = svc.decrypt_secret(&nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_nonces_per_encryption() {
        let svc = CryptoService::from_key(&test_key());
        let plaintext = b"same_data";

        let (nonce1, ct1) = svc.encrypt_secret(plaintext).unwrap();
        let (nonce2, ct2) = svc.encrypt_secret(plaintext).unwrap();

        // Nonces should differ (random)
        assert_ne!(nonce1, nonce2);
        // Ciphertexts should differ due to different nonces
        assert_ne!(ct1, ct2);

        // Both should decrypt correctly
        assert_eq!(svc.decrypt_secret(&nonce1, &ct1).unwrap(), plaintext);
        assert_eq!(svc.decrypt_secret(&nonce2, &ct2).unwrap(), plaintext);
    }

    #[test]
    #[ignore = "modifies env vars, not safe in parallel"]
    fn test_from_env_missing_key() {
        std::env::remove_var("ENCRYPTION_MASTER_KEY");
        let result = CryptoService::from_env();
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "modifies env vars, not safe in parallel"]
    fn test_from_env_invalid_hex() {
        std::env::set_var("ENCRYPTION_MASTER_KEY", "not_hex!");
        let result = CryptoService::from_env();
        assert!(result.is_err());
        std::env::remove_var("ENCRYPTION_MASTER_KEY");
    }

    #[test]
    #[ignore = "modifies env vars, not safe in parallel"]
    fn test_from_env_wrong_length() {
        // 16 bytes = 32 hex chars (too short, need 64)
        std::env::set_var("ENCRYPTION_MASTER_KEY", "00112233445566778899aabbccddeeff");
        let result = CryptoService::from_env();
        assert!(result.is_err());
        std::env::remove_var("ENCRYPTION_MASTER_KEY");
    }

    #[test]
    #[ignore = "modifies env vars, not safe in parallel"]
    fn test_from_env_valid() {
        let hex_key = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        std::env::set_var("ENCRYPTION_MASTER_KEY", hex_key);
        let result = CryptoService::from_env();
        assert!(result.is_ok());
        std::env::remove_var("ENCRYPTION_MASTER_KEY");
    }
}
