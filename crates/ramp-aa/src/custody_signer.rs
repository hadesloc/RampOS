use std::sync::Arc;

use alloy::primitives::Address;
use ramp_core::custody::MpcSigningService;

use crate::user_operation::UserOperation;

/// Custody signer integrates MPC threshold signing into ERC-4337 UserOperation flow.
pub struct CustodySigner {
    signing_service: Arc<MpcSigningService>,
}

impl CustodySigner {
    pub fn new(signing_service: Arc<MpcSigningService>) -> Self {
        Self { signing_service }
    }

    /// Sign a UserOperation hash via simulated 2-of-3 MPC threshold workflow.
    pub fn sign_user_operation(
        &self,
        user_id: &str,
        user_op: &UserOperation,
        entry_point: Address,
        chain_id: u64,
    ) -> ramp_common::Result<Vec<u8>> {
        let op_hash = user_op.hash(entry_point, chain_id);
        self.sign_hash(user_id, op_hash.as_slice())
    }

    /// Sign an arbitrary 32-byte hash via MPC workflow.
    pub fn sign_hash(&self, user_id: &str, hash: &[u8]) -> ramp_common::Result<Vec<u8>> {
        let session = self
            .signing_service
            .create_signing_request(user_id, hash.to_vec())?;

        // Simulated approval from second party in a 2-of-3 scheme.
        // Party 1 is auto-approved when request is created.
        self.signing_service
            .approve_signing(&session.id, 2, hash.to_vec())?;

        self.signing_service.combine_signatures(&session.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{Address, Bytes, U256};
    use ramp_core::custody::MpcSigningService;

    use crate::user_operation::UserOperation;

    #[test]
    fn test_sign_hash_returns_32_bytes_signature() {
        let signer = CustodySigner::new(Arc::new(MpcSigningService::new()));
        let hash = [7u8; 32];

        let signature = signer.sign_hash("user-1", &hash).unwrap();

        assert_eq!(signature.len(), 32);
    }

    #[test]
    fn test_sign_user_operation_integration() {
        let signer = CustodySigner::new(Arc::new(MpcSigningService::new()));

        let user_op = UserOperation {
            sender: Address::ZERO,
            nonce: U256::from(1),
            init_code: Bytes::default(),
            call_data: Bytes::from(vec![0x01, 0x02]),
            call_gas_limit: U256::from(100_000),
            verification_gas_limit: U256::from(100_000),
            pre_verification_gas: U256::from(21_000),
            max_fee_per_gas: U256::from(1_000_000_000u64),
            max_priority_fee_per_gas: U256::from(1_000_000_000u64),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::default(),
        };

        let signature = signer
            .sign_user_operation("user-1", &user_op, Address::ZERO, 1)
            .unwrap();

        assert_eq!(signature.len(), 32);
    }
}
