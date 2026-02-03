use crate::user_operation::UserOperation;
use ethers::prelude::*;
use ethers::types::{transaction::eip2718::TypedTransaction, Address, Bytes, U256};
use ramp_common::Result;
use std::sync::Arc;
// use ethers::providers::Middleware; // Unused
use ethers::providers::Middleware;

// Removed orphan impl From<ProviderError> for ramp_common::Error
// impl From<ProviderError> for ramp_common::Error { ... }

/// Gas estimate result
#[derive(Debug, Clone, Default)]
pub struct GasEstimate {
    pub verification_gas: U256,
    pub call_gas: U256,
    pub pre_verification_gas: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
}

/// Gas estimator for UserOperations
pub struct GasEstimator<M: Middleware> {
    provider: Arc<M>,
    entry_point_address: Address,
}

impl<M: Middleware> GasEstimator<M> {
    pub fn new(provider: Arc<M>, entry_point_address: Address) -> Self {
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
        self.provider
            .get_gas_price()
            .await
            .map_err(|e| ramp_common::Error::Provider(e.to_string()))
    }

    /// Estimate pre-verification gas (overhead)
    /// Based on calldata size and fixed overheads
    fn estimate_pre_verification_gas_internal(&self, user_op: &UserOperation) -> u64 {
        let mut gas: u64 = 21000; // Fixed overhead

        // Packed UserOperation overhead (approximate)
        // In v0.6/0.7, we calculate based on packed data

        // Calldata cost
        // 16 gas per non-zero byte, 4 gas per zero byte

        // Helper to calc bytes cost
        let calc_bytes_cost =
            |bytes: &Bytes| -> u64 { bytes.iter().map(|&b| if b == 0 { 4 } else { 16 }).sum() };

        gas += calc_bytes_cost(&user_op.call_data);
        gas += calc_bytes_cost(&user_op.init_code);
        gas += calc_bytes_cost(&user_op.paymaster_and_data);
        gas += calc_bytes_cost(&user_op.signature);

        // Add userOp fixed fields overhead (approximate based on packed size)
        // sender (20), nonce (32), gas limits (32), fees (32)
        // 20 + 32 + 32 + 32 = 116 bytes. Assuming mostly non-zero for safety
        gas += 116 * 16;

        // Add safety buffer (10%)
        gas + (gas / 10)
    }

    /// Estimate call gas by simulating the call
    pub async fn estimate_call_gas(&self, user_op: &UserOperation) -> Result<U256> {
        // If init_code is present, we can't easily simulate the call directly to sender
        // because sender might not exist yet.
        // For accurate estimation with init_code, we'd need to simulate via EntryPoint or
        // assume a standard cost for deployment + call.

        // Construct the transaction
        let tx = Eip1559TransactionRequest::new()
            .from(self.entry_point_address) // Pretend to be EntryPoint
            .to(user_op.sender)
            .data(user_op.call_data.clone());

        // We don't set value as UserOps don't transfer value from EntryPoint directly in the same way,
        // unless it's an execute call.

        // Estimate gas
        let gas = self
            .provider
            .estimate_gas(&TypedTransaction::Eip1559(tx), None)
            .await
            .map_err(|e| ramp_common::Error::Provider(e.to_string()))?;

        let gas_u64 = gas.as_u64();
        // Add safety buffer (10%)
        Ok(U256::from(gas_u64 + (gas_u64 / 10)))
    }

    /// Estimate verification gas
    /// This is difficult to do accurately without EntryPoint simulation support.
    /// We'll use a heuristic or simulate validation if possible.
    /// For this task, we'll implement a simplified approach or placeholder logic
    /// as full simulation requires handling EntryPoint reverts.
    pub async fn estimate_verification_gas(&self, _user_op: &UserOperation) -> Result<U256> {
        // In a real implementation, this would call `eth_estimateUserOperationGas` on the bundler
        // or simulate `validateUserOp` via `eth_call`.

        // Since we are implementing the estimator, we might be the ones providing data for the bundler
        // or a client trying to guess.

        // Let's assume a safe default for basic verification if we can't simulate
        // 100,000 is a standard buffer for verification
        let base_verification = 100_000;

        Ok(U256::from(base_verification))
    }

    /// Estimate current fees
    async fn estimate_fees(&self) -> Result<(U256, U256)> {
        let (max_fee, max_priority) = self
            .provider
            .estimate_eip1559_fees(None)
            .await
            .map_err(|e| ramp_common::Error::Provider(e.to_string()))?;

        Ok((max_fee, max_priority))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::providers::Provider;

    #[tokio::test]
    async fn test_estimate_pre_verification_gas() {
        let (client, _mock) = Provider::mocked();
        let provider = Arc::new(client);
        let estimator = GasEstimator::new(provider, Address::zero());

        let user_op = UserOperation::new(
            Address::zero(),
            U256::zero(),
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
