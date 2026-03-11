//! Passkey signer for ERC-4337 UserOperation signing
//!
//! Encodes WebAuthn P256 signatures into the format expected by
//! RampOSAccount._validatePasskeySignature():
//!   [SIG_TYPE_PASSKEY(1 byte) || r(32 bytes) || s(32 bytes)]
//!
//! The signature type byte (0x01) tells the smart contract to route
//! verification to the P256 path instead of ECDSA.

use alloy::primitives::{Bytes, U256};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::user_operation::UserOperation;

/// Signature type byte for passkey signatures
const SIG_TYPE_PASSKEY: u8 = 0x01;

/// Signature type byte for ECDSA signatures
const SIG_TYPE_ECDSA: u8 = 0x00;

/// P256 signature components from WebAuthn assertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P256Signature {
    /// The r component of the ECDSA signature (32 bytes)
    pub r: [u8; 32],
    /// The s component of the ECDSA signature (32 bytes)
    pub s: [u8; 32],
}

/// WebAuthn assertion data from the authenticator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnAssertion {
    /// The authenticator data from the WebAuthn response
    pub authenticator_data: Vec<u8>,
    /// The client data JSON from the WebAuthn response
    pub client_data_json: Vec<u8>,
    /// The P256 signature
    pub signature: P256Signature,
    /// The credential ID used for this assertion
    pub credential_id: String,
}

/// Passkey public key (P256 coordinates)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyPublicKey {
    /// X coordinate of the P256 public key (hex-encoded, 32 bytes)
    pub x: String,
    /// Y coordinate of the P256 public key (hex-encoded, 32 bytes)
    pub y: String,
}

impl PasskeyPublicKey {
    /// Parse x coordinate as U256
    pub fn x_as_u256(&self) -> Result<U256, String> {
        let cleaned = self.x.strip_prefix("0x").unwrap_or(&self.x);
        let bytes = hex::decode(cleaned).map_err(|e| format!("Invalid hex for x: {}", e))?;
        if bytes.len() > 32 {
            return Err("X coordinate too long".to_string());
        }
        let mut padded = [0u8; 32];
        padded[32 - bytes.len()..].copy_from_slice(&bytes);
        Ok(U256::from_be_bytes(padded))
    }

    /// Parse y coordinate as U256
    pub fn y_as_u256(&self) -> Result<U256, String> {
        let cleaned = self.y.strip_prefix("0x").unwrap_or(&self.y);
        let bytes = hex::decode(cleaned).map_err(|e| format!("Invalid hex for y: {}", e))?;
        if bytes.len() > 32 {
            return Err("Y coordinate too long".to_string());
        }
        let mut padded = [0u8; 32];
        padded[32 - bytes.len()..].copy_from_slice(&bytes);
        Ok(U256::from_be_bytes(padded))
    }
}

/// Passkey signer - encodes WebAuthn assertions for on-chain verification
pub struct PasskeySigner {
    /// The passkey public key
    pub public_key: PasskeyPublicKey,
}

impl PasskeySigner {
    /// Create a new PasskeySigner with a public key
    pub fn new(public_key: PasskeyPublicKey) -> Self {
        Self { public_key }
    }

    /// Encode a passkey signature for use in an ERC-4337 UserOperation
    ///
    /// Format: [0x01 || r(32 bytes) || s(32 bytes)]
    ///
    /// The 0x01 prefix tells RampOSAccount._validateSignature() to route
    /// to the P256 verification path.
    pub fn encode_signature(&self, signature: &P256Signature) -> Bytes {
        let mut encoded = Vec::with_capacity(65);
        encoded.push(SIG_TYPE_PASSKEY);
        encoded.extend_from_slice(&signature.r);
        encoded.extend_from_slice(&signature.s);
        Bytes::from(encoded)
    }

    /// Encode an ECDSA signature with explicit type prefix
    ///
    /// Format: [0x00 || signature(65 bytes)]
    pub fn encode_ecdsa_signature(signature: &[u8]) -> Bytes {
        let mut encoded = Vec::with_capacity(1 + signature.len());
        encoded.push(SIG_TYPE_ECDSA);
        encoded.extend_from_slice(signature);
        Bytes::from(encoded)
    }

    /// Sign a UserOperation with a passkey signature
    ///
    /// Takes the WebAuthn assertion from the client and encodes it
    /// into the UserOperation signature field.
    pub fn sign_user_operation(
        &self,
        user_op: &mut UserOperation,
        assertion: &WebAuthnAssertion,
    ) -> Result<(), String> {
        let encoded = self.encode_signature(&assertion.signature);

        info!(
            credential_id = %assertion.credential_id,
            sig_length = encoded.len(),
            "Passkey signature encoded for UserOperation"
        );

        user_op.signature = encoded;
        Ok(())
    }

    /// Build the calldata for `setPasskeySigner(uint256, uint256)` on RampOSAccount
    ///
    /// Returns the ABI-encoded calldata to set this passkey as a signer
    /// on a RampOSAccount smart contract.
    pub fn build_set_passkey_calldata(&self) -> Result<Bytes, String> {
        let x = self.public_key.x_as_u256()?;
        let y = self.public_key.y_as_u256()?;

        // Function selector for setPasskeySigner(uint256,uint256)
        // keccak256("setPasskeySigner(uint256,uint256)") = 0x...
        // We compute it manually
        let selector = &alloy::primitives::keccak256(b"setPasskeySigner(uint256,uint256)")[..4];

        let mut calldata = Vec::with_capacity(68);
        calldata.extend_from_slice(selector);

        // ABI encode the two uint256 parameters
        let x_bytes = x.to_be_bytes::<32>();
        let y_bytes = y.to_be_bytes::<32>();
        calldata.extend_from_slice(&x_bytes);
        calldata.extend_from_slice(&y_bytes);

        Ok(Bytes::from(calldata))
    }

    /// Get the public key coordinates as U256 values
    pub fn get_public_key_u256(&self) -> Result<(U256, U256), String> {
        let x = self.public_key.x_as_u256()?;
        let y = self.public_key.y_as_u256()?;
        Ok((x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::Address;

    fn test_public_key() -> PasskeyPublicKey {
        PasskeyPublicKey {
            x: "6B17D1F2E12C4247F8BCE6E563A440F277037D812DEB33A0F4A13945D898C296".to_string(),
            y: "4FE342E2FE1A7F9B8EE7EB4A7C0F9E162BCE33576B315ECECBB6406837BF51F5".to_string(),
        }
    }

    fn test_signature() -> P256Signature {
        P256Signature {
            r: [
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
                0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
                0x1d, 0x1e, 0x1f, 0x20,
            ],
            s: [
                0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e,
                0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c,
                0x3d, 0x3e, 0x3f, 0x40,
            ],
        }
    }

    #[test]
    fn test_encode_passkey_signature() {
        let signer = PasskeySigner::new(test_public_key());
        let sig = test_signature();

        let encoded = signer.encode_signature(&sig);

        // Should be 65 bytes: 1 (type) + 32 (r) + 32 (s)
        assert_eq!(encoded.len(), 65);
        assert_eq!(encoded[0], SIG_TYPE_PASSKEY);
        assert_eq!(&encoded[1..33], &sig.r);
        assert_eq!(&encoded[33..65], &sig.s);
    }

    #[test]
    fn test_encode_ecdsa_signature() {
        let ecdsa_sig = vec![0u8; 65]; // Standard ECDSA signature
        let encoded = PasskeySigner::encode_ecdsa_signature(&ecdsa_sig);

        assert_eq!(encoded.len(), 66); // 1 (type) + 65 (sig)
        assert_eq!(encoded[0], SIG_TYPE_ECDSA);
    }

    #[test]
    fn test_public_key_to_u256() {
        let pk = test_public_key();
        let x = pk.x_as_u256().unwrap();
        let y = pk.y_as_u256().unwrap();

        assert!(x > U256::ZERO);
        assert!(y > U256::ZERO);
    }

    #[test]
    fn test_public_key_with_0x_prefix() {
        let pk = PasskeyPublicKey {
            x: "0x6B17D1F2E12C4247F8BCE6E563A440F277037D812DEB33A0F4A13945D898C296".to_string(),
            y: "0x4FE342E2FE1A7F9B8EE7EB4A7C0F9E162BCE33576B315ECECBB6406837BF51F5".to_string(),
        };

        let x = pk.x_as_u256().unwrap();
        assert!(x > U256::ZERO);
    }

    #[test]
    fn test_public_key_invalid_hex() {
        let pk = PasskeyPublicKey {
            x: "GGGG".to_string(),
            y: "1234".to_string(),
        };

        assert!(pk.x_as_u256().is_err());
    }

    #[test]
    fn test_sign_user_operation() {
        let signer = PasskeySigner::new(test_public_key());
        let sender: Address = "0x1234567890abcdef1234567890abcdef12345678"
            .parse()
            .unwrap();
        let mut user_op = UserOperation::new(sender, U256::ZERO, Bytes::default());

        let assertion = WebAuthnAssertion {
            authenticator_data: vec![0u8; 37],
            client_data_json: b"{\"type\":\"webauthn.get\"}".to_vec(),
            signature: test_signature(),
            credential_id: "test-cred".to_string(),
        };

        let result = signer.sign_user_operation(&mut user_op, &assertion);
        assert!(result.is_ok());
        assert_eq!(user_op.signature.len(), 65);
        assert_eq!(user_op.signature[0], SIG_TYPE_PASSKEY);
    }

    #[test]
    fn test_build_set_passkey_calldata() {
        let signer = PasskeySigner::new(test_public_key());
        let calldata = signer.build_set_passkey_calldata().unwrap();

        // Should be 4 (selector) + 32 (x) + 32 (y) = 68 bytes
        assert_eq!(calldata.len(), 68);
    }

    #[test]
    fn test_get_public_key_u256() {
        let signer = PasskeySigner::new(test_public_key());
        let (x, y) = signer.get_public_key_u256().unwrap();

        assert!(x > U256::ZERO);
        assert!(y > U256::ZERO);
    }

    #[test]
    fn test_signature_type_constants() {
        assert_eq!(SIG_TYPE_PASSKEY, 0x01);
        assert_eq!(SIG_TYPE_ECDSA, 0x00);
    }
}
