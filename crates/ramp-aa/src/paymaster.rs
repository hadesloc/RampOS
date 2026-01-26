use async_trait::async_trait;
use chrono::Utc;
use ethers::types::{Address, Bytes, U256};
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
pub struct PaymasterService {
    paymaster_address: Address,
    signer_key: Vec<u8>, // In production, would use HSM/Vault
}

impl PaymasterService {
    pub fn new(paymaster_address: Address, signer_key: Vec<u8>) -> Self {
        Self {
            paymaster_address,
            signer_key,
        }
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

    /// Sign paymaster data
    fn sign_paymaster_data(&self, user_op_hash: &[u8], valid_until: u64, valid_after: u64) -> Vec<u8> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut data = Vec::new();
        data.extend_from_slice(user_op_hash);
        data.extend_from_slice(&valid_until.to_be_bytes());
        data.extend_from_slice(&valid_after.to_be_bytes());

        let mut mac = HmacSha256::new_from_slice(&self.signer_key).unwrap();
        mac.update(&data);
        mac.finalize().into_bytes().to_vec()
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
        let now = Utc::now().timestamp() as u64;

        if now < paymaster_data.valid_after {
            return Ok(false);
        }

        if now > paymaster_data.valid_until {
            return Ok(false);
        }

        // In production, would verify signature
        Ok(true)
    }
}
