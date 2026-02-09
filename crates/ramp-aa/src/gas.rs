use crate::user_operation::UserOperation;
use alloy::primitives::{Address, Bytes, U256};
use async_trait::async_trait;
use ramp_common::Result;
use std::sync::Arc;

/// Gas estimate result
#[derive(Debug, Clone, Default)]
pub struct GasEstimate {
    pub verification_gas: U256,
    pub call_gas: U256,
    pub pre_verification_gas: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
}

/// Provider abstraction for gas estimation
/// Replaces direct provider middleware dependency (uses alloy::providers)
#[async_trait]
pub trait GasProvider: Send + Sync {
    /// Get current gas price
    async fn get_gas_price(&self) -> Result<U256>;

    /// Estimate gas for a call
    async fn estimate_gas(&self, from: Address, to: Address, data: Bytes) -> Result<U256>;

    /// Estimate EIP-1559 fees (max_fee_per_gas, max_priority_fee_per_gas)
    async fn estimate_eip1559_fees(&self) -> Result<(U256, U256)>;
}

/// Gas estimator for UserOperations
pub struct GasEstimator<P: GasProvider> {
    provider: Arc<P>,
    entry_point_address: Address,
}

impl<P: GasProvider> GasEstimator<P> {
    pub fn new(provider: Arc<P>, entry_point_address: Address) -> Self {
        Self {
            provider,
            entry_point_address,
        }
    }

    /// Estimate gas for a UserOperation
    pub async fn estimate_user_op_gas(&self, user_op: &UserOperation) -> Result<GasEstimate> {
        let pre_verification_gas = self.estimate_pre_verification_gas_internal(user_op);
        let call_gas = self.estimate_call_gas(user_op).await?;
        let verification_gas = self.estimate_verification_gas(user_op).await?;

        let (max_fee_per_gas, max_priority_fee_per_gas) = self.estimate_fees().await?;

        Ok(GasEstimate {
            pre_verification_gas: U256::from(pre_verification_gas),
            verification_gas,
            call_gas,
            max_fee_per_gas,
            max_priority_fee_per_gas,
        })
    }

    /// Get current gas price
    pub async fn get_gas_price(&self) -> Result<U256> {
        self.provider.get_gas_price().await
    }

    /// Estimate pre-verification gas (overhead)
    /// Based on calldata size and fixed overheads
    fn estimate_pre_verification_gas_internal(&self, user_op: &UserOperation) -> u64 {
        let mut gas: u64 = 21000; // Fixed overhead

        // Calldata cost
        // 16 gas per non-zero byte, 4 gas per zero byte
        let calc_bytes_cost =
            |bytes: &[u8]| -> u64 { bytes.iter().map(|&b| if b == 0 { 4 } else { 16 }).sum() };

        gas += calc_bytes_cost(user_op.call_data.as_ref());
        gas += calc_bytes_cost(user_op.init_code.as_ref());
        gas += calc_bytes_cost(user_op.paymaster_and_data.as_ref());
        gas += calc_bytes_cost(user_op.signature.as_ref());

        // Add userOp fixed fields overhead (approximate based on packed size)
        // sender (20), nonce (32), gas limits (32), fees (32)
        // 20 + 32 + 32 + 32 = 116 bytes. Assuming mostly non-zero for safety
        gas += 116 * 16;

        // Add safety buffer (10%)
        gas + (gas / 10)
    }

    /// Estimate call gas by simulating the call
    pub async fn estimate_call_gas(&self, user_op: &UserOperation) -> Result<U256> {
        let gas = self
            .provider
            .estimate_gas(
                self.entry_point_address,
                user_op.sender,
                user_op.call_data.clone(),
            )
            .await?;

        let gas_u64: u64 = gas.try_into().unwrap_or(u64::MAX);
        // Add safety buffer (10%)
        Ok(U256::from(gas_u64 + (gas_u64 / 10)))
    }

    /// Estimate verification gas
    pub async fn estimate_verification_gas(&self, _user_op: &UserOperation) -> Result<U256> {
        let base_verification = 100_000;
        Ok(U256::from(base_verification))
    }

    /// Estimate current fees
    async fn estimate_fees(&self) -> Result<(U256, U256)> {
        self.provider.estimate_eip1559_fees().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock provider for testing
    struct MockProvider;

    #[async_trait]
    impl GasProvider for MockProvider {
        async fn get_gas_price(&self) -> Result<U256> {
            Ok(U256::from(1_000_000_000u64)) // 1 gwei
        }

        async fn estimate_gas(&self, _from: Address, _to: Address, _data: Bytes) -> Result<U256> {
            Ok(U256::from(21000u64))
        }

        async fn estimate_eip1559_fees(&self) -> Result<(U256, U256)> {
            Ok((U256::from(20_000_000_000u64), U256::from(1_000_000_000u64)))
        }
    }

    #[tokio::test]
    async fn test_estimate_pre_verification_gas() {
        let provider = Arc::new(MockProvider);
        let estimator = GasEstimator::new(provider, Address::ZERO);

        let user_op = UserOperation::new(
            Address::ZERO,
            U256::ZERO,
            Bytes::from(vec![1, 2, 3, 0]), // 3 non-zero (16*3=48), 1 zero (4) = 52
        );

        let gas = estimator.estimate_pre_verification_gas_internal(&user_op);

        // Calculation:
        // Fixed: 21000
        // Calldata: 48 + 4 = 52
        // InitCode: 0
        // Paymaster: 0
        // Signature: 0
        // Fixed Fields: 116 * 16 = 1856
        // Total Base: 21000 + 52 + 1856 = 22908
        // Buffer: 22908 / 10 = 2290
        // Total: 22908 + 2290 = 25198

        assert_eq!(gas, 25198);
    }
}
