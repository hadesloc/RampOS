//! Webhook Signature v2 Service (F04.04)
//!
//! Provides Ed25519 signing for webhook payloads alongside the existing
//! HMAC-SHA256 v1 signatures. This module supports dual-mode signing
//! where both v1 and v2 signatures can be included for backward compatibility.
//!
//! Header format:
//!   `RampOS-Signature-V2: t={timestamp},ed25519={hex_signature}`
//!
//! The signed message is: `{timestamp}.{payload_bytes}`

use chrono::Utc;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use ramp_common::Result;
use serde::{Deserialize, Serialize};

/// Webhook signing key pair for Ed25519
#[derive(Clone)]
pub struct WebhookSigningKeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl WebhookSigningKeyPair {
    /// Generate a new random Ed25519 key pair
    pub fn generate() -> Self {
        let mut csprng = rand::thread_rng();
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Create from a 32-byte secret seed
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Get the public verifying key bytes (for sharing with webhook consumers)
    pub fn verifying_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get the public verifying key as hex string
    pub fn verifying_key_hex(&self) -> String {
        hex::encode(self.verifying_key.to_bytes())
    }
}

impl std::fmt::Debug for WebhookSigningKeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookSigningKeyPair")
            .field("verifying_key", &self.verifying_key_hex())
            .finish()
    }
}

/// Signature version enum for backward compatibility
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureVersion {
    /// HMAC-SHA256 (existing)
    V1,
    /// Ed25519 (new)
    V2,
}

/// Result of a v2 signature operation
#[derive(Debug, Clone)]
pub struct SignatureV2Result {
    /// The full header value: `t={timestamp},ed25519={hex_signature}`
    pub header_value: String,
    /// The timestamp used
    pub timestamp: i64,
    /// The raw signature bytes
    pub signature_bytes: Vec<u8>,
}

/// Webhook Signing Service provides Ed25519-based signature generation
/// and verification for webhook payloads.
pub struct WebhookSigningService {
    key_pair: WebhookSigningKeyPair,
}

impl WebhookSigningService {
    /// Create a new signing service with a generated key pair
    pub fn new() -> Self {
        Self {
            key_pair: WebhookSigningKeyPair::generate(),
        }
    }

    /// Create with a specific key pair
    pub fn with_key_pair(key_pair: WebhookSigningKeyPair) -> Self {
        Self { key_pair }
    }

    /// Create from a 32-byte seed
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        Self {
            key_pair: WebhookSigningKeyPair::from_seed(seed),
        }
    }

    /// Sign a webhook payload with Ed25519 (Signature v2)
    ///
    /// The signed message format is: `{timestamp}.{payload}`
    /// The header format is: `t={timestamp},ed25519={hex_signature}`
    pub fn sign_v2(&self, payload: &[u8]) -> Result<SignatureV2Result> {
        let timestamp = Utc::now().timestamp();
        self.sign_v2_with_timestamp(payload, timestamp)
    }

    /// Sign with a specific timestamp (for testing)
    pub fn sign_v2_with_timestamp(
        &self,
        payload: &[u8],
        timestamp: i64,
    ) -> Result<SignatureV2Result> {
        let signed_message = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

        let signature = self.key_pair.signing_key.sign(signed_message.as_bytes());
        let sig_hex = hex::encode(signature.to_bytes());

        let header_value = format!("t={},ed25519={}", timestamp, sig_hex);

        Ok(SignatureV2Result {
            header_value,
            timestamp,
            signature_bytes: signature.to_bytes().to_vec(),
        })
    }

    /// Verify a v2 signature
    ///
    /// Parses the header format `t={timestamp},ed25519={hex_signature}`
    /// and verifies the signature against the payload.
    pub fn verify_v2(
        &self,
        signature_header: &str,
        payload: &[u8],
        tolerance_secs: i64,
    ) -> std::result::Result<i64, WebhookSignatureV2Error> {
        let (timestamp, sig_bytes) = parse_v2_header(signature_header)?;

        // Check timestamp tolerance
        let now = Utc::now().timestamp();
        if (now - timestamp).abs() > tolerance_secs {
            return Err(WebhookSignatureV2Error::TimestampOutOfRange);
        }

        // Reconstruct signed message
        let signed_message = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

        // Verify signature
        let signature = Signature::from_bytes(&sig_bytes);
        self.key_pair
            .verifying_key
            .verify(signed_message.as_bytes(), &signature)
            .map_err(|_| WebhookSignatureV2Error::SignatureMismatch)?;

        Ok(timestamp)
    }

    /// Verify a v2 signature with an external verifying key
    pub fn verify_v2_with_key(
        verifying_key_bytes: &[u8; 32],
        signature_header: &str,
        payload: &[u8],
        tolerance_secs: i64,
    ) -> std::result::Result<i64, WebhookSignatureV2Error> {
        let verifying_key = VerifyingKey::from_bytes(verifying_key_bytes)
            .map_err(|_| WebhookSignatureV2Error::InvalidKey)?;

        let (timestamp, sig_bytes) = parse_v2_header(signature_header)?;

        // Check timestamp tolerance
        let now = Utc::now().timestamp();
        if (now - timestamp).abs() > tolerance_secs {
            return Err(WebhookSignatureV2Error::TimestampOutOfRange);
        }

        let signed_message = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

        let signature = Signature::from_bytes(&sig_bytes);
        verifying_key
            .verify(signed_message.as_bytes(), &signature)
            .map_err(|_| WebhookSignatureV2Error::SignatureMismatch)?;

        Ok(timestamp)
    }

    /// Get the public key for sharing with webhook consumers
    pub fn public_key_hex(&self) -> String {
        self.key_pair.verifying_key_hex()
    }

    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.key_pair.verifying_key_bytes()
    }

    /// Generate both v1 (HMAC-SHA256) and v2 (Ed25519) signatures
    /// for backward-compatible dual signing.
    pub fn sign_dual(&self, hmac_secret: &[u8], payload: &[u8]) -> Result<DualSignatureResult> {
        let timestamp = Utc::now().timestamp();

        // Generate v1 signature (HMAC-SHA256)
        let v1_header =
            ramp_common::crypto::generate_webhook_signature(hmac_secret, timestamp, payload)?;

        // Generate v2 signature (Ed25519)
        let v2_result = self.sign_v2_with_timestamp(payload, timestamp)?;

        Ok(DualSignatureResult {
            v1_header,
            v2_header: v2_result.header_value,
            timestamp,
        })
    }
}

impl Default for WebhookSigningService {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of dual (v1 + v2) signature generation
#[derive(Debug, Clone)]
pub struct DualSignatureResult {
    /// v1 header: `t={timestamp},v1={hmac_hex}`
    pub v1_header: String,
    /// v2 header: `t={timestamp},ed25519={ed25519_hex}`
    pub v2_header: String,
    /// The timestamp used for both signatures
    pub timestamp: i64,
}

/// Parse the v2 signature header format
fn parse_v2_header(header: &str) -> std::result::Result<(i64, [u8; 64]), WebhookSignatureV2Error> {
    let parts: std::collections::HashMap<&str, &str> = header
        .split(',')
        .filter_map(|part| {
            let mut iter = part.splitn(2, '=');
            Some((iter.next()?, iter.next()?))
        })
        .collect();

    let timestamp: i64 = parts
        .get("t")
        .ok_or(WebhookSignatureV2Error::MissingTimestamp)?
        .parse()
        .map_err(|_| WebhookSignatureV2Error::InvalidTimestamp)?;

    let sig_hex = parts
        .get("ed25519")
        .ok_or(WebhookSignatureV2Error::MissingSignature)?;

    let sig_bytes = hex::decode(sig_hex).map_err(|_| WebhookSignatureV2Error::InvalidSignature)?;

    if sig_bytes.len() != 64 {
        return Err(WebhookSignatureV2Error::InvalidSignature);
    }

    let mut sig_array = [0u8; 64];
    sig_array.copy_from_slice(&sig_bytes);

    Ok((timestamp, sig_array))
}

/// Errors specific to v2 signature operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookSignatureV2Error {
    MissingTimestamp,
    InvalidTimestamp,
    MissingSignature,
    InvalidSignature,
    InvalidKey,
    TimestampOutOfRange,
    SignatureMismatch,
}

impl std::fmt::Display for WebhookSignatureV2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTimestamp => write!(f, "Missing timestamp in v2 signature"),
            Self::InvalidTimestamp => write!(f, "Invalid timestamp format in v2 signature"),
            Self::MissingSignature => write!(f, "Missing Ed25519 signature"),
            Self::InvalidSignature => write!(f, "Invalid Ed25519 signature format"),
            Self::InvalidKey => write!(f, "Invalid Ed25519 verifying key"),
            Self::TimestampOutOfRange => write!(f, "Timestamp out of tolerance range"),
            Self::SignatureMismatch => write!(f, "Ed25519 signature verification failed"),
        }
    }
}

impl std::error::Error for WebhookSignatureV2Error {}
