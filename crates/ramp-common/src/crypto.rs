use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

use crate::Result;
use crate::Error;

/// Generate HMAC-SHA256 signature for webhook
///
/// # Errors
///
/// Returns `Error::Internal` if the secret key is invalid for HMAC-SHA256.
pub fn hmac_sha256(secret: &[u8], message: &[u8]) -> Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|_| Error::Internal("Invalid HMAC key length".to_string()))?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().to_vec())
}

/// Verify HMAC-SHA256 signature
///
/// # Errors
///
/// Returns `Error::Internal` if the secret key is invalid for HMAC-SHA256.
pub fn verify_hmac_sha256(secret: &[u8], message: &[u8], signature: &[u8]) -> Result<bool> {
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|_| Error::Internal("Invalid HMAC key length".to_string()))?;
    mac.update(message);
    Ok(mac.verify_slice(signature).is_ok())
}

/// Generate webhook signature header value
/// Format: t=<timestamp>,v1=<signature>
pub fn generate_webhook_signature(secret: &[u8], timestamp: i64, payload: &[u8]) -> Result<String> {
    let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));
    let signature = hmac_sha256(secret, signed_payload.as_bytes())?;
    Ok(format!("t={},v1={}", timestamp, hex::encode(signature)))
}

/// Parse and verify webhook signature
/// Returns Ok(timestamp) if valid, Err otherwise
///
/// # Errors
///
/// Returns `WebhookSignatureError` if:
/// - Timestamp is missing or invalid
/// - Signature is missing or invalid hex
/// - Timestamp is outside tolerance window
/// - Signature verification fails
pub fn verify_webhook_signature(
    secret: &[u8],
    signature_header: &str,
    payload: &[u8],
    tolerance_secs: i64,
) -> std::result::Result<i64, WebhookSignatureError> {
    // Parse header: t=<timestamp>,v1=<signature>
    let parts: std::collections::HashMap<&str, &str> = signature_header
        .split(',')
        .filter_map(|part| {
            let mut iter = part.splitn(2, '=');
            Some((iter.next()?, iter.next()?))
        })
        .collect();

    let timestamp: i64 = parts
        .get("t")
        .ok_or(WebhookSignatureError::MissingTimestamp)?
        .parse()
        .map_err(|_| WebhookSignatureError::InvalidTimestamp)?;

    let signature_hex = parts
        .get("v1")
        .ok_or(WebhookSignatureError::MissingSignature)?;

    let signature =
        hex::decode(signature_hex).map_err(|_| WebhookSignatureError::InvalidSignature)?;

    // Check timestamp tolerance (prevent replay attacks)
    let now = chrono::Utc::now().timestamp();
    if (now - timestamp).abs() > tolerance_secs {
        return Err(WebhookSignatureError::TimestampOutOfRange);
    }

    // Verify signature
    let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

    // We can't easily propagate errors here because verify_webhook_signature returns specific WebhookSignatureError
    // But verify_hmac_sha256 now returns Result<bool, Error>
    // So we map the generic error to a signature mismatch (safe default)
    if !verify_hmac_sha256(secret, signed_payload.as_bytes(), &signature)
        .unwrap_or(false)
    {
        return Err(WebhookSignatureError::SignatureMismatch);
    }

    Ok(timestamp)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookSignatureError {
    MissingTimestamp,
    InvalidTimestamp,
    MissingSignature,
    InvalidSignature,
    TimestampOutOfRange,
    SignatureMismatch,
}

impl std::fmt::Display for WebhookSignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTimestamp => write!(f, "Missing timestamp in signature"),
            Self::InvalidTimestamp => write!(f, "Invalid timestamp format"),
            Self::MissingSignature => write!(f, "Missing signature"),
            Self::InvalidSignature => write!(f, "Invalid signature format"),
            Self::TimestampOutOfRange => write!(f, "Timestamp out of tolerance range"),
            Self::SignatureMismatch => write!(f, "Signature verification failed"),
        }
    }
}

impl std::error::Error for WebhookSignatureError {}

/// Hash payload using SHA-256
#[must_use]
pub fn sha256_hash(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sha256() {
        let secret = b"test_secret";
        let message = b"test_message";
        let sig = hmac_sha256(secret, message).expect("hmac failed");
        assert!(verify_hmac_sha256(secret, message, &sig).expect("verify failed"));
    }

    #[test]
    fn test_webhook_signature() {
        let secret = b"whsec_test123";
        let payload = br#"{"event":"test"}"#;
        let timestamp = chrono::Utc::now().timestamp();

        let signature = generate_webhook_signature(secret, timestamp, payload).expect("generate failed");
        let result = verify_webhook_signature(secret, &signature, payload, 300);

        assert!(result.is_ok());
        assert_eq!(result.expect("verify failed"), timestamp);
    }
}
