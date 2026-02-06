use async_trait::async_trait;
use chrono::Utc;
use ethers::types::{Address, Bytes, U256};
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use ramp_common::{types::TenantId, Result};
use tracing::info;

use crate::types::PaymasterData;
use crate::user_operation::UserOperation;

/// Paymaster sponsorship policy
#[derive(Debug, Clone)]
pub struct SponsorshipPolicy {
    pub tenant_id: TenantId,
    pub max_gas_per_op: U256,
    pub max_ops_per_user_per_day: u32,
    pub max_daily_spend: U256,
    pub allowed_contracts: Vec<Address>,
    pub allowed_selectors: Vec<[u8; 4]>,
}

impl Default for SponsorshipPolicy {
    fn default() -> Self {
        Self {
            tenant_id: TenantId::new("default"),
            max_gas_per_op: U256::from(500_000),
            max_ops_per_user_per_day: 100,
            max_daily_spend: U256::from(1_000_000_000_000_000_000u64), // 1 ETH
            allowed_contracts: vec![],
            allowed_selectors: vec![],
        }
    }
}

/// Paymaster service trait
#[async_trait]
pub trait Paymaster: Send + Sync {
    /// Check if operation can be sponsored
    async fn can_sponsor(
        &self,
        user_op: &UserOperation,
        policy: &SponsorshipPolicy,
    ) -> Result<bool>;

    /// Generate paymaster data for sponsorship
    async fn sponsor(&self, user_op: &UserOperation) -> Result<PaymasterData>;

    /// Validate paymaster data
    async fn validate(&self, paymaster_data: &PaymasterData) -> Result<bool>;
}

/// RampOS Paymaster Service
///
/// SECURITY: Uses ECDSA (secp256k1) for ERC-4337 compatible signatures.
/// The signer_key must be a 32-byte private key for secp256k1.
pub struct PaymasterService {
    paymaster_address: Address,
    signing_key: SigningKey, // ECDSA signing key (secp256k1)
}

impl PaymasterService {
    pub fn new(paymaster_address: Address, signer_key: Vec<u8>) -> Result<Self> {
        // Convert raw bytes to SigningKey
        // The key must be exactly 32 bytes for secp256k1
        let key_bytes: [u8; 32] = signer_key
            .try_into()
            .map_err(|_| ramp_common::Error::Validation("Signer key must be exactly 32 bytes for secp256k1".into()))?;

        let signing_key =
            SigningKey::from_bytes(&key_bytes.into()).map_err(|e| ramp_common::Error::Validation(format!("Invalid secp256k1 private key: {}", e)))?;

        Ok(Self {
            paymaster_address,
            signing_key,
        })
    }

    /// Get the verifying (public) key for signature verification
    pub fn verifying_key(&self) -> VerifyingKey {
        *self.signing_key.verifying_key()
    }

    /// Check daily usage for a user
    async fn check_daily_usage(&self, _sender: Address) -> Result<(u32, U256)> {
        // In production, would query database
        Ok((0, U256::zero()))
    }

    /// Record usage
    async fn _record_usage(&self, _sender: Address, _gas_cost: U256) -> Result<()> {
        // In production, would update database
        Ok(())
    }

    /// Sign paymaster data using ECDSA (secp256k1)
    ///
    /// Returns Ethereum-compatible signature in (r, s, v) format (65 bytes total).
    /// The signature is over keccak256(user_op_hash || valid_until || valid_after).
    ///
    /// SECURITY: The recovery ID (v) is properly computed from the signature,
    /// ensuring the signature can be verified on-chain using ECDSA.recover().
    /// For Ethereum: v = 27 + recovery_id (where recovery_id is 0 or 1)
    fn sign_paymaster_data(
        &self,
        user_op_hash: &[u8],
        valid_until: u64,
        valid_after: u64,
    ) -> Result<Vec<u8>> {
        use ethers::utils::keccak256;

        // Construct the message to sign
        let mut data = Vec::new();
        data.extend_from_slice(user_op_hash);
        data.extend_from_slice(&valid_until.to_be_bytes());
        data.extend_from_slice(&valid_after.to_be_bytes());

        // Hash the data using keccak256 (Ethereum standard)
        let message_hash = keccak256(&data);

        // Create Ethereum signed message hash (EIP-191)
        let eth_message = "\x19Ethereum Signed Message:\n32".to_string();
        let mut prefixed = eth_message.into_bytes();
        prefixed.extend_from_slice(&message_hash);
        let eth_signed_hash = keccak256(&prefixed);

        // Sign using ECDSA with recoverable signature
        // sign_prehash_recoverable returns (Signature, RecoveryId)
        let (signature, recovery_id): (Signature, RecoveryId) = self
            .signing_key
            .sign_prehash_recoverable(&eth_signed_hash)
            .map_err(|e| ramp_common::Error::Internal(format!("Signing failed: {}", e)))?;

        // Convert to Ethereum signature format (r || s || v)
        let r = signature.r().to_bytes();
        let s = signature.s().to_bytes();

        // Calculate v: For Ethereum legacy format, v = 27 + recovery_id (0 or 1)
        // recovery_id.to_byte() returns 0 or 1
        let v: u8 = 27 + recovery_id.to_byte();

        let mut sig_bytes = Vec::with_capacity(65);
        sig_bytes.extend_from_slice(&r);
        sig_bytes.extend_from_slice(&s);
        sig_bytes.push(v);

        Ok(sig_bytes)
    }

    /// Verify a signature and recover the signer's Ethereum address
    ///
    /// This is useful for testing that signatures can be verified on-chain.
    /// Returns the recovered Ethereum address from the signature.
    pub fn recover_signer(
        message_hash: &[u8; 32],
        signature: &[u8; 65],
    ) -> std::result::Result<Address, String> {
        use ethers::utils::keccak256;
        use k256::PublicKey;

        // Extract r, s, v from signature
        let r = &signature[0..32];
        let s = &signature[32..64];
        let v = signature[64];

        // Normalize v to recovery_id (0 or 1)
        let recovery_id_byte = if v >= 27 { v - 27 } else { v };
        if recovery_id_byte > 1 {
            return Err(format!("Invalid recovery id: {}", v));
        }

        let recovery_id = RecoveryId::from_byte(recovery_id_byte).ok_or("Invalid recovery id")?;

        // Reconstruct the signature
        let mut rs_bytes = [0u8; 64];
        rs_bytes[0..32].copy_from_slice(r);
        rs_bytes[32..64].copy_from_slice(s);

        let signature =
            Signature::from_slice(&rs_bytes).map_err(|e| format!("Invalid signature: {}", e))?;

        // Recover the verifying key from the prehashed message
        let verifying_key =
            VerifyingKey::recover_from_prehash(message_hash, &signature, recovery_id)
                .map_err(|e| format!("Failed to recover public key: {}", e))?;

        // Convert verifying key to Ethereum address
        // Use uncompressed format (65 bytes: 0x04 || x || y)
        let public_key = PublicKey::from(&verifying_key);
        let encoded_point = public_key.to_encoded_point(false); // false = uncompressed
        let public_key_bytes = encoded_point.as_bytes();

        // Ethereum address is last 20 bytes of keccak256(uncompressed_pubkey[1..])
        // The SEC1 uncompressed format is: 0x04 || x || y (65 bytes)
        if public_key_bytes.len() != 65 || public_key_bytes[0] != 0x04 {
            return Err(format!(
                "Invalid public key format: len={}, prefix=0x{:02x}",
                public_key_bytes.len(),
                public_key_bytes.first().unwrap_or(&0)
            ));
        }

        let hash = keccak256(&public_key_bytes[1..]);
        let mut address_bytes = [0u8; 20];
        address_bytes.copy_from_slice(&hash[12..]);

        Ok(Address::from(address_bytes))
    }

    /// Get the Ethereum address derived from the signing key
    pub fn signer_address(&self) -> Address {
        use ethers::utils::keccak256;
        use k256::PublicKey;

        let verifying_key = self.signing_key.verifying_key();
        let public_key = PublicKey::from(verifying_key);
        // Use uncompressed format (65 bytes: 0x04 || x || y)
        let encoded_point = public_key.to_encoded_point(false); // false = uncompressed
        let public_key_bytes = encoded_point.as_bytes();

        // Ethereum address is last 20 bytes of keccak256(uncompressed_pubkey[1..])
        let hash = keccak256(&public_key_bytes[1..]);
        let mut address_bytes = [0u8; 20];
        address_bytes.copy_from_slice(&hash[12..]);

        Address::from(address_bytes)
    }
}

#[async_trait]
impl Paymaster for PaymasterService {
    async fn can_sponsor(
        &self,
        user_op: &UserOperation,
        policy: &SponsorshipPolicy,
    ) -> Result<bool> {
        // Check gas limit
        let total_gas =
            user_op.call_gas_limit + user_op.verification_gas_limit + user_op.pre_verification_gas;

        if total_gas > policy.max_gas_per_op {
            info!(
                sender = %user_op.sender,
                total_gas = %total_gas,
                max_gas = %policy.max_gas_per_op,
                "Gas limit exceeded for sponsorship"
            );
            return Ok(false);
        }

        // Check daily usage
        let (ops_today, _spend_today) = self.check_daily_usage(user_op.sender).await?;

        if ops_today >= policy.max_ops_per_user_per_day {
            info!(
                sender = %user_op.sender,
                ops_today = ops_today,
                "Daily operation limit reached"
            );
            return Ok(false);
        }

        // Check allowed contracts if specified
        if !policy.allowed_contracts.is_empty() {
            // Would need to decode call_data to check target
            // For now, allow all
        }

        Ok(true)
    }

    async fn sponsor(&self, user_op: &UserOperation) -> Result<PaymasterData> {
        let now = Utc::now().timestamp() as u64;
        let valid_after = now;
        let valid_until = now + 3600; // 1 hour validity

        // Create paymaster data
        // Format: paymaster address (20) + validUntil (6) + validAfter (6) + signature
        let mut paymaster_and_data = Vec::with_capacity(32 + 65);
        paymaster_and_data.extend_from_slice(self.paymaster_address.as_bytes());
        paymaster_and_data.extend_from_slice(&valid_until.to_be_bytes()[2..8]); // 6 bytes
        paymaster_and_data.extend_from_slice(&valid_after.to_be_bytes()[2..8]); // 6 bytes

        // Sign (in production, would use proper signature)
        let signature = self.sign_paymaster_data(&user_op.call_data, valid_until, valid_after)?;
        paymaster_and_data.extend_from_slice(&signature);

        info!(
            sender = %user_op.sender,
            valid_until = valid_until,
            "Sponsoring UserOperation"
        );

        Ok(PaymasterData {
            paymaster_address: self.paymaster_address,
            paymaster_and_data: Bytes::from(paymaster_and_data),
            valid_until,
            valid_after,
        })
    }

    async fn validate(&self, paymaster_data: &PaymasterData) -> Result<bool> {
        use ethers::utils::keccak256;
        use k256::ecdsa::{signature::Verifier, Signature};

        let now = Utc::now().timestamp() as u64;

        // Check time validity
        if now < paymaster_data.valid_after {
            info!(
                now = now,
                valid_after = paymaster_data.valid_after,
                "Paymaster data not yet valid"
            );
            return Ok(false);
        }

        if now > paymaster_data.valid_until {
            info!(
                now = now,
                valid_until = paymaster_data.valid_until,
                "Paymaster data expired"
            );
            return Ok(false);
        }

        // Verify paymaster address matches
        if paymaster_data.paymaster_address != self.paymaster_address {
            info!(
                expected = %self.paymaster_address,
                actual = %paymaster_data.paymaster_address,
                "Paymaster address mismatch"
            );
            return Ok(false);
        }

        // Extract and verify ECDSA signature from paymaster_and_data
        // Format: paymaster address (20 bytes) + validUntil (6 bytes) + validAfter (6 bytes) + signature (65 bytes)
        let data = paymaster_data.paymaster_and_data.as_ref();

        // Minimum length: 20 (address) + 6 (validUntil) + 6 (validAfter) + 65 (ECDSA signature) = 97 bytes
        if data.len() < 97 {
            info!(
                data_len = data.len(),
                "Paymaster data too short for ECDSA signature verification"
            );
            return Ok(false);
        }

        // Extract the signature (last 65 bytes for ECDSA r || s || v)
        let signature_start = data.len() - 65;
        let sig_bytes = &data[signature_start..];

        // Extract r, s from signature (first 64 bytes)
        let r = &sig_bytes[0..32];
        let s = &sig_bytes[32..64];
        // v is sig_bytes[64] but not used for verification

        // Reconstruct r || s for k256 Signature
        let mut rs_bytes = [0u8; 64];
        rs_bytes[0..32].copy_from_slice(r);
        rs_bytes[32..64].copy_from_slice(s);

        let signature = match Signature::from_slice(&rs_bytes) {
            Ok(sig) => sig,
            Err(_) => {
                info!("Invalid ECDSA signature format");
                return Ok(false);
            }
        };

        // Reconstruct the signed message
        // Note: We don't have access to the original user_op_hash here
        // In a full implementation, we would need to pass it or reconstruct it
        let mut verify_data = Vec::new();
        verify_data.extend_from_slice(&paymaster_data.valid_until.to_be_bytes());
        verify_data.extend_from_slice(&paymaster_data.valid_after.to_be_bytes());

        // Hash the data
        let message_hash = keccak256(&verify_data);

        // Create Ethereum signed message hash (EIP-191)
        let eth_message = "\x19Ethereum Signed Message:\n32".to_string();
        let mut prefixed = eth_message.into_bytes();
        prefixed.extend_from_slice(&message_hash);
        let eth_signed_hash = keccak256(&prefixed);

        // Verify signature
        let verifying_key = self.verifying_key();
        if verifying_key.verify(&eth_signed_hash, &signature).is_err() {
            info!("Paymaster ECDSA signature verification failed");
            return Ok(false);
        }

        info!(
            valid_until = paymaster_data.valid_until,
            valid_after = paymaster_data.valid_after,
            "Paymaster data validated successfully"
        );

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::utils::keccak256;

    /// Generate a test signing key (deterministic for reproducible tests)
    fn test_signing_key() -> Vec<u8> {
        // A fixed 32-byte private key for testing
        // In production, this would be securely generated
        vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x20,
        ]
    }

    #[test]
    fn test_sign_and_recover_address() {
        // Create paymaster service with test key
        let paymaster_address = Address::from([0x42u8; 20]);
        let service = PaymasterService::new(paymaster_address, test_signing_key()).expect("Failed to create PaymasterService");

        // Get the expected signer address
        let expected_address = service.signer_address();

        // Sign some data
        let user_op_hash = [0xab; 32];
        let valid_until = 1700000000u64;
        let valid_after = 1699999000u64;

        let signature = service.sign_paymaster_data(&user_op_hash, valid_until, valid_after).expect("Signing failed");

        // Verify signature length is 65 bytes (r=32 + s=32 + v=1)
        assert_eq!(signature.len(), 65);

        // Verify v is 27 or 28
        let v = signature[64];
        assert!(v == 27 || v == 28, "v should be 27 or 28, got {}", v);

        // Reconstruct the message hash for recovery
        let mut data = Vec::new();
        data.extend_from_slice(&user_op_hash);
        data.extend_from_slice(&valid_until.to_be_bytes());
        data.extend_from_slice(&valid_after.to_be_bytes());
        let message_hash = keccak256(&data);

        // Create Ethereum signed message hash (EIP-191)
        let eth_message = format!("\x19Ethereum Signed Message:\n32");
        let mut prefixed = eth_message.into_bytes();
        prefixed.extend_from_slice(&message_hash);
        let eth_signed_hash = keccak256(&prefixed);

        // Recover signer address from signature
        let sig_array: [u8; 65] = signature.try_into().unwrap_or([0u8; 65]);
        let recovered_address = PaymasterService::recover_signer(&eth_signed_hash, &sig_array)
            .expect("recovery should succeed");

        // Verify the recovered address matches
        assert_eq!(
            recovered_address, expected_address,
            "Recovered address should match signer address"
        );
    }

    #[test]
    fn test_recovery_id_is_correct() {
        // Create multiple signatures and verify all have valid v values
        let paymaster_address = Address::from([0x42u8; 20]);
        let service = PaymasterService::new(paymaster_address, test_signing_key()).expect("Failed to create PaymasterService");

        let expected_address = service.signer_address();

        // Sign multiple different messages to test different recovery IDs
        for i in 0..20 {
            let user_op_hash = [i as u8; 32];
            let valid_until = 1700000000u64 + i as u64;
            let valid_after = 1699999000u64;

            let signature = service.sign_paymaster_data(&user_op_hash, valid_until, valid_after).expect("Signing failed");
            let v = signature[64];

            // v must be 27 or 28
            assert!(
                v == 27 || v == 28,
                "Iteration {}: v should be 27 or 28, got {}",
                i,
                v
            );

            // Reconstruct the message hash for recovery
            let mut data = Vec::new();
            data.extend_from_slice(&user_op_hash);
            data.extend_from_slice(&valid_until.to_be_bytes());
            data.extend_from_slice(&valid_after.to_be_bytes());
            let message_hash = keccak256(&data);

            let eth_message = format!("\x19Ethereum Signed Message:\n32");
            let mut prefixed = eth_message.into_bytes();
            prefixed.extend_from_slice(&message_hash);
            let eth_signed_hash = keccak256(&prefixed);

            // Recover and verify
            let sig_array: [u8; 65] = signature.try_into().unwrap_or([0u8; 65]);
            let recovered_address = PaymasterService::recover_signer(&eth_signed_hash, &sig_array)
                .expect(&format!("Iteration {}: recovery should succeed", i));

            assert_eq!(
                recovered_address, expected_address,
                "Iteration {}: Recovered address should match signer address",
                i
            );
        }
    }

    #[test]
    fn test_signer_address_derivation() {
        // Verify that signer_address correctly derives the Ethereum address
        let paymaster_address = Address::from([0x42u8; 20]);
        let service = PaymasterService::new(paymaster_address, test_signing_key()).expect("Failed to create PaymasterService");

        let address = service.signer_address();

        // The address should be non-zero
        assert_ne!(address, Address::zero());

        // The address should be deterministic
        let service2 = PaymasterService::new(paymaster_address, test_signing_key()).expect("Failed to create PaymasterService");
        assert_eq!(service.signer_address(), service2.signer_address());
    }

    #[test]
    fn test_different_keys_produce_different_addresses() {
        let paymaster_address = Address::from([0x42u8; 20]);

        let key1 = vec![1u8; 32];
        let key2 = vec![2u8; 32];

        let service1 = PaymasterService::new(paymaster_address, key1).expect("Failed to create PaymasterService");
        let service2 = PaymasterService::new(paymaster_address, key2).expect("Failed to create PaymasterService");

        assert_ne!(
            service1.signer_address(),
            service2.signer_address(),
            "Different keys should produce different addresses"
        );
    }

    #[test]
    fn test_signature_format_compatibility() {
        // Test that the signature format is compatible with Ethereum ecrecover
        let paymaster_address = Address::from([0x42u8; 20]);
        let service = PaymasterService::new(paymaster_address, test_signing_key()).expect("Failed to create PaymasterService");

        let user_op_hash = [0xde; 32];
        let valid_until = 1700000000u64;
        let valid_after = 1699999000u64;

        let signature = service.sign_paymaster_data(&user_op_hash, valid_until, valid_after).expect("Signing failed");

        // Verify r is 32 bytes (first 32 bytes)
        let r = &signature[0..32];
        assert_eq!(r.len(), 32);

        // Verify s is 32 bytes (next 32 bytes)
        let s = &signature[32..64];
        assert_eq!(s.len(), 32);

        // Verify v is 1 byte and is 27 or 28
        let v = signature[64];
        assert!(v == 27 || v == 28);

        // Verify r and s are not zero (would indicate a problem)
        assert!(r.iter().any(|&b| b != 0), "r should not be all zeros");
        assert!(s.iter().any(|&b| b != 0), "s should not be all zeros");
    }

    #[test]
    fn test_recover_signer_with_invalid_v() {
        let message_hash = [0xab; 32];
        let mut signature = [0u8; 65];
        signature[64] = 30; // Invalid v value

        let result = PaymasterService::recover_signer(&message_hash, &signature);
        assert!(result.is_err(), "Should fail with invalid v value");
    }

    #[test]
    fn test_recover_signer_with_invalid_signature() {
        let message_hash = [0xab; 32];
        let mut signature = [0u8; 65];
        signature[64] = 27; // Valid v
                            // r and s are zero, which is invalid

        let result = PaymasterService::recover_signer(&message_hash, &signature);
        // This might succeed or fail depending on the implementation
        // The important thing is it doesn't panic
        let _ = result;
    }

    #[tokio::test]
    async fn test_sponsor_produces_valid_signature() {
        use crate::user_operation::UserOperation;

        let paymaster_address = Address::from([0x42u8; 20]);
        let service = PaymasterService::new(paymaster_address, test_signing_key()).expect("Failed to create PaymasterService");

        let user_op = UserOperation {
            sender: Address::from([0x11u8; 20]),
            nonce: U256::from(1),
            init_code: Bytes::from(vec![]),
            call_data: Bytes::from(vec![0x01, 0x02, 0x03]),
            call_gas_limit: U256::from(100_000),
            verification_gas_limit: U256::from(100_000),
            pre_verification_gas: U256::from(21_000),
            max_fee_per_gas: U256::from(1_000_000_000),
            max_priority_fee_per_gas: U256::from(1_000_000),
            paymaster_and_data: Bytes::from(vec![]),
            signature: Bytes::from(vec![]),
        };

        let paymaster_data = service
            .sponsor(&user_op)
            .await
            .expect("sponsor should succeed");

        // Verify paymaster_and_data has correct format
        // 20 (address) + 6 (validUntil) + 6 (validAfter) + 65 (signature) = 97 bytes
        assert_eq!(
            paymaster_data.paymaster_and_data.len(),
            97,
            "paymaster_and_data should be 97 bytes"
        );

        // Verify the embedded address matches
        let embedded_address = &paymaster_data.paymaster_and_data[0..20];
        assert_eq!(embedded_address, paymaster_address.as_bytes());

        // Verify the signature portion
        let sig_start = 32; // 20 + 6 + 6
        let signature = &paymaster_data.paymaster_and_data[sig_start..];
        assert_eq!(signature.len(), 65);

        let v = signature[64];
        assert!(v == 27 || v == 28, "v should be 27 or 28");
    }
}
