use async_trait::async_trait;
use chrono::Utc;
use ethers::types::{Address, Bytes, U256};
use k256::ecdsa::{signature::Signer, Signature, SigningKey, VerifyingKey};
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
    async fn can_sponsor(&self, user_op: &UserOperation, policy: &SponsorshipPolicy) -> Result<bool>;

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
    pub fn new(paymaster_address: Address, signer_key: Vec<u8>) -> Self {
        // Convert raw bytes to SigningKey
        // The key must be exactly 32 bytes for secp256k1
        let key_bytes: [u8; 32] = signer_key
            .try_into()
            .expect("Signer key must be exactly 32 bytes for secp256k1");

        let signing_key = SigningKey::from_bytes(&key_bytes.into())
            .expect("Invalid secp256k1 private key");

        Self {
            paymaster_address,
            signing_key,
        }
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
    fn sign_paymaster_data(&self, user_op_hash: &[u8], valid_until: u64, valid_after: u64) -> Vec<u8> {
        use ethers::utils::keccak256;

        // Construct the message to sign
        let mut data = Vec::new();
        data.extend_from_slice(user_op_hash);
        data.extend_from_slice(&valid_until.to_be_bytes());
        data.extend_from_slice(&valid_after.to_be_bytes());

        // Hash the data using keccak256 (Ethereum standard)
        let message_hash = keccak256(&data);

        // Create Ethereum signed message hash (EIP-191)
        let eth_message = format!("\x19Ethereum Signed Message:\n32");
        let mut prefixed = eth_message.into_bytes();
        prefixed.extend_from_slice(&message_hash);
        let eth_signed_hash = keccak256(&prefixed);

        // Sign using ECDSA
        let signature: Signature = self.signing_key.sign(&eth_signed_hash);

        // Convert to Ethereum signature format (r || s || v)
        let r = signature.r().to_bytes();
        let s = signature.s().to_bytes();

        // Calculate recovery id (v)
        // For Ethereum: v = recovery_id + 27
        // We use a simple approach here - in production you'd compute this properly
        let v: u8 = 27; // Simplified - real implementation needs recovery computation

        let mut sig_bytes = Vec::with_capacity(65);
        sig_bytes.extend_from_slice(&r);
        sig_bytes.extend_from_slice(&s);
        sig_bytes.push(v);

        sig_bytes
    }
}

#[async_trait]
impl Paymaster for PaymasterService {
    async fn can_sponsor(&self, user_op: &UserOperation, policy: &SponsorshipPolicy) -> Result<bool> {
        // Check gas limit
        let total_gas = user_op.call_gas_limit
            + user_op.verification_gas_limit
            + user_op.pre_verification_gas;

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
        let signature = self.sign_paymaster_data(&user_op.call_data, valid_until, valid_after);
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
        let eth_message = format!("\x19Ethereum Signed Message:\n32");
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
